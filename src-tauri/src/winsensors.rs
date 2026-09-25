#![allow(dead_code)]

use crate::smc::{Fan, SmcSensors, SmcTemp};
use crate::util::{run, run_res};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::Manager;

const HELPER: &str = "coreview-sensors.exe";

#[derive(Deserialize, Clone, Default)]
struct Sensor {
    hw: String,
    ht: String,
    st: String,
    name: String,
    value: Option<f64>,
    max: Option<f64>,
}

#[derive(Deserialize, Default)]
struct Line {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    admin: bool,
    #[serde(default)]
    error: String,
    #[serde(default)]
    sensors: Vec<Sensor>,
}

#[derive(Default, Clone)]
struct Snapshot {
    sensors: Vec<Sensor>,
    admin: bool,
    error: String,
    running: bool,
    found: bool,
}

static SNAP: Mutex<Snapshot> = Mutex::new(Snapshot { sensors: Vec::new(), admin: false, error: String::new(), running: false, found: false });
static STARTED: OnceLock<()> = OnceLock::new();

fn update(f: impl FnOnce(&mut Snapshot)) {
    if let Ok(mut s) = SNAP.lock() {
        f(&mut s);
    }
}

fn candidates(resource: Option<PathBuf>) -> Vec<PathBuf> {
    let mut list = Vec::new();
    if let Some(r) = resource {
        list.push(r.join("sensors").join(HELPER));
        list.push(r.join(HELPER));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        list.push(dir.join("sensors").join(HELPER));
        list.push(dir.join("resources").join("sensors").join(HELPER));
        for up in ["../../../sensors-helper/out", "../../../../sensors-helper/out"] {
            list.push(dir.join(up).join(HELPER));
        }
    }
    list
}

fn run_once(exe: &Path) -> Result<(), String> {
    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let _keep_open = child.stdin.take();
    let out = child.stdout.take().ok_or("no output from sensor helper")?;
    update(|s| s.running = true);
    for line in BufReader::new(out).lines() {
        let Ok(line) = line else { break };
        if let Ok(parsed) = serde_json::from_str::<Line>(&line) {
            update(|s| {
                s.admin = parsed.admin;
                if parsed.ok {
                    s.sensors = parsed.sensors;
                    s.error.clear();
                } else {
                    s.error = parsed.error;
                }
            });
        }
    }
    update(|s| s.running = false);
    let _ = child.wait();
    Ok(())
}

fn supervise(resource: Option<PathBuf>) {
    let Some(exe) = candidates(resource).into_iter().find(|p| p.exists()) else {
        update(|s| s.error = "sensor helper not found".into());
        return;
    };
    update(|s| s.found = true);
    for _ in 0..8 {
        if let Err(e) = run_once(&exe) {
            update(|s| s.error = e);
        }
        std::thread::sleep(Duration::from_secs(3));
    }
}

pub fn ensure_started(app: &tauri::AppHandle) {
    if STARTED.set(()).is_err() {
        return;
    }
    let resource = app.path().resource_dir().ok();
    std::thread::spawn(move || supervise(resource));
}

fn group_of(hardware: &str) -> &'static str {
    match hardware {
        "Cpu" => "cpu",
        "GpuNvidia" | "GpuAmd" | "GpuIntel" => "gpu",
        "Storage" => "storage",
        "Motherboard" | "SuperIO" | "EmbeddedController" => "board",
        "Memory" => "memory",
        "Battery" => "battery",
        _ => "other",
    }
}

pub fn to_smc() -> SmcSensors {
    let snap = SNAP.lock().map(|s| s.clone()).unwrap_or_default();
    let mut out = SmcSensors { supported: snap.found, ..Default::default() };
    for s in &snap.sensors {
        let Some(value) = s.value else { continue };
        match s.st.as_str() {
            "Temperature" if value > 0.0 && value < 150.0 => {
                let lower = s.name.to_lowercase();
                let (kind, index) = if let Some(rest) = lower.strip_prefix("core #") {
                    ("core", rest.trim().parse::<u32>().ok())
                } else if lower == "cpu package" {
                    ("package", None)
                } else {
                    ("", None)
                };
                out.temps.push(SmcTemp {
                    key: format!("{}|{}|{}", s.ht, s.hw, s.name),
                    group: group_of(&s.ht).into(),
                    kind: kind.into(),
                    index,
                    celsius: value,
                    name: s.name.clone(),
                    hw: s.hw.clone(),
                });
            }
            "Fan" => out.fans.push(Fan {
                id: out.fans.len() as u32,
                rpm: value,
                min: 0.0,
                max: s.max.unwrap_or(0.0),
                name: if s.hw.is_empty() || s.name.to_lowercase().contains("fan") && group_of(&s.ht) == "gpu" { s.name.clone() } else { s.name.clone() },
            }),
            _ => {}
        }
    }
    out
}

#[derive(Default)]
pub struct WinPower {
    pub cpu: Option<f64>,
    pub gpu: Option<f64>,
    pub gpu_load: Option<f64>,
}

pub fn to_power() -> WinPower {
    let snap = SNAP.lock().map(|s| s.clone()).unwrap_or_default();
    let mut out = WinPower::default();
    for s in &snap.sensors {
        let Some(v) = s.value else { continue };
        let gpu = s.ht.starts_with("Gpu");
        match (s.st.as_str(), s.ht.as_str()) {
            ("Power", "Cpu") if s.name.contains("Package") => out.cpu = out.cpu.or(Some(v)),
            ("Power", _) if gpu && (s.name.contains("Package") || s.name == "GPU Power") => out.gpu = out.gpu.or(Some(v)),
            ("Load", _) if gpu && s.name == "GPU Core" => out.gpu_load = out.gpu_load.or(Some(v)),
            _ => {}
        }
    }
    out
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorStatus {
    platform: String,
    admin: bool,
    driver: bool,
    helper: String,
    error: String,
    sensor_count: usize,
}

fn driver_installed() -> bool {
    run("sc", &["query", "PawnIO"]).is_some()
}

#[tauri::command]
pub async fn sensor_status(app: tauri::AppHandle) -> SensorStatus {
    tauri::async_runtime::spawn_blocking(move || {
        if !cfg!(windows) {
            return SensorStatus { platform: std::env::consts::OS.into(), admin: true, driver: true, helper: "native".into(), error: String::new(), sensor_count: 0 };
        }
        ensure_started(&app);
        let snap = SNAP.lock().map(|s| s.clone()).unwrap_or_default();
        SensorStatus {
            platform: "windows".into(),
            admin: snap.admin,
            driver: driver_installed(),
            helper: if snap.running { "running" } else if snap.found { "stopped" } else { "missing" }.into(),
            error: snap.error,
            sensor_count: snap.sensors.len(),
        }
    })
    .await
    .unwrap_or(SensorStatus { platform: "windows".into(), admin: false, driver: false, helper: "missing".into(), error: String::new(), sensor_count: 0 })
}

#[tauri::command]
pub async fn install_sensor_driver() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        run_res(
            "winget",
            &["install", "--id", "namazso.PawnIO", "-e", "--silent", "--accept-package-agreements", "--accept-source-agreements"],
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
