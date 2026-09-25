#![allow(dead_code)]

use crate::smc::{Fan, SmcSensors, SmcTemp};
use crate::util::{run, run_res};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
static NVIDIA: Mutex<Vec<GpuStats>> = Mutex::new(Vec::new());
static ENGINE_LOAD: Mutex<Option<f64>> = Mutex::new(None);

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpuStats {
    pub name: String,
    pub load: Option<f64>,
    pub temp: Option<f64>,
    pub hotspot: Option<f64>,
    pub fan_rpm: Option<f64>,
    pub fan_percent: Option<f64>,
    pub power: Option<f64>,
    pub core_mhz: Option<f64>,
    pub mem_mhz: Option<f64>,
    pub mem_used: Option<f64>,
    pub mem_total: Option<f64>,
}

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
    std::thread::spawn(gpu_fallbacks);
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
            "Fan" if value > 0.0 => out.fans.push(Fan {
                id: out.fans.len() as u32,
                rpm: value,
                min: 0.0,
                max: s.max.unwrap_or(0.0),
                name: if group_of(&s.ht) == "gpu" { format!("{} · {}", s.hw, s.name) } else { s.name.clone() },
                percent: None,
            }),
            _ => {}
        }
    }
    let with_rpm: Vec<&str> = snap.sensors.iter().filter(|s| s.st == "Fan" && s.value.unwrap_or(0.0) > 0.0).map(|s| s.hw.as_str()).collect();
    for s in snap.sensors.iter().filter(|s| s.st == "Control" && s.name.to_lowercase().contains("fan") && !with_rpm.contains(&s.hw.as_str())) {
        if let Some(v) = s.value {
            out.fans.push(Fan {
                id: out.fans.len() as u32,
                name: if group_of(&s.ht) == "gpu" { format!("{} · {}", s.hw, s.name) } else { s.name.clone() },
                percent: Some(v),
                ..Default::default()
            });
        }
    }
    if !snap.sensors.iter().any(|s| s.ht.starts_with("Gpu") && (s.st == "Fan" || s.st == "Control")) {
        for g in NVIDIA.lock().map(|n| n.clone()).unwrap_or_default() {
            if let Some(p) = g.fan_percent {
                out.fans.push(Fan { id: out.fans.len() as u32, name: format!("{} · GPU", g.name), percent: Some(p), ..Default::default() });
            }
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
        if let (Some(v), "Power", "Cpu") = (s.value, s.st.as_str(), s.ht.as_str()) {
            if s.name.contains("Package") {
                out.cpu = out.cpu.or(Some(v));
            }
        }
    }
    if let Some(g) = gpus().first() {
        out.gpu = g.power;
        out.gpu_load = g.load;
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

fn number(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok()
}

fn nvidia_smi() -> Option<PathBuf> {
    let mut list = vec![PathBuf::from("nvidia-smi.exe")];
    if let Ok(root) = std::env::var("SystemRoot") {
        list.push(PathBuf::from(&root).join("System32").join("nvidia-smi.exe"));
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        list.push(PathBuf::from(pf).join("NVIDIA Corporation").join("NVSMI").join("nvidia-smi.exe"));
    }
    list.into_iter().find(|p| run(&p.to_string_lossy(), &["--version"]).is_some())
}

fn read_nvidia(exe: &Path) -> Vec<GpuStats> {
    let query = "name,utilization.gpu,temperature.gpu,fan.speed,power.draw,clocks.gr,clocks.mem,memory.used,memory.total";
    let Some(text) = run(&exe.to_string_lossy(), &[&format!("--query-gpu={query}"), "--format=csv,noheader,nounits"]) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(',').map(|c| c.trim()).collect();
            (f.len() >= 9).then(|| GpuStats {
                name: f[0].to_string(),
                load: number(f[1]),
                temp: number(f[2]),
                fan_percent: number(f[3]),
                power: number(f[4]),
                core_mhz: number(f[5]),
                mem_mhz: number(f[6]),
                mem_used: number(f[7]),
                mem_total: number(f[8]),
                ..Default::default()
            })
        })
        .collect()
}

fn gpu_fallbacks() {
    let smi = nvidia_smi();
    #[cfg(windows)]
    let mut engines = crate::gpuperf::GpuEngines::new();
    loop {
        if let Some(exe) = &smi {
            let list = read_nvidia(exe);
            if let Ok(mut n) = NVIDIA.lock() {
                *n = list;
            }
        }
        #[cfg(windows)]
        if let Some(e) = engines.as_mut() {
            let value = e.read();
            if let Ok(mut l) = ENGINE_LOAD.lock() {
                *l = value;
            }
        }
        std::thread::sleep(Duration::from_millis(1500));
    }
}

fn pick<'a>(list: &'a [&Sensor], st: &str, names: &[&str]) -> Option<&'a Sensor> {
    for n in names {
        if let Some(s) = list.iter().find(|s| s.st == st && s.name.eq_ignore_ascii_case(n) && s.value.is_some()) {
            return Some(*s);
        }
    }
    None
}

pub fn gpus() -> Vec<GpuStats> {
    let snap = SNAP.lock().map(|s| s.clone()).unwrap_or_default();
    let mut groups: BTreeMap<String, Vec<&Sensor>> = BTreeMap::new();
    for s in snap.sensors.iter().filter(|s| s.ht.starts_with("Gpu")) {
        groups.entry(s.hw.clone()).or_default().push(s);
    }
    let nvidia = NVIDIA.lock().map(|n| n.clone()).unwrap_or_default();
    let engine = ENGINE_LOAD.lock().ok().and_then(|l| *l);

    let mut out: Vec<GpuStats> = groups
        .iter()
        .map(|(name, list)| {
            let val = |st: &str, names: &[&str]| pick(list, st, names).and_then(|s| s.value);
            let any = |st: &str| list.iter().filter(|s| s.st == st).find_map(|s| s.value);
            let mut g = GpuStats {
                name: name.clone(),
                load: val("Load", &["GPU Core", "D3D 3D", "GPU Core Load"]).or_else(|| list.iter().filter(|s| s.st == "Load" && (s.name.contains("3D") || s.name.contains("Core"))).find_map(|s| s.value)),
                temp: val("Temperature", &["GPU Core", "GPU Temperature"]).or_else(|| any("Temperature")),
                hotspot: val("Temperature", &["GPU Hot Spot", "GPU Hotspot"]),
                fan_rpm: list.iter().filter(|s| s.st == "Fan").find_map(|s| s.value),
                fan_percent: list.iter().filter(|s| s.st == "Control" && s.name.to_lowercase().contains("fan")).find_map(|s| s.value),
                power: val("Power", &["GPU Package", "GPU Power", "GPU Total"]).or_else(|| any("Power")),
                core_mhz: val("Clock", &["GPU Core"]),
                mem_mhz: val("Clock", &["GPU Memory"]),
                mem_used: val("SmallData", &["GPU Memory Used", "D3D Dedicated Memory Used"]),
                mem_total: val("SmallData", &["GPU Memory Total", "D3D Dedicated Memory Total"]),
            };
            if let Some(n) = nvidia.iter().find(|n| name.contains(&n.name) || n.name.contains(name.as_str())).or_else(|| (nvidia.len() == 1 && list.iter().any(|s| s.ht == "GpuNvidia")).then(|| &nvidia[0])) {
                g.load = n.load.or(g.load);
                g.temp = g.temp.or(n.temp);
                g.fan_percent = g.fan_percent.or(n.fan_percent);
                g.power = g.power.or(n.power);
                g.core_mhz = g.core_mhz.or(n.core_mhz);
                g.mem_mhz = g.mem_mhz.or(n.mem_mhz);
                g.mem_used = g.mem_used.or(n.mem_used);
                g.mem_total = g.mem_total.or(n.mem_total);
            }
            g
        })
        .collect();

    if out.is_empty() {
        out = nvidia.clone();
    }
    if out.is_empty() && engine.is_some() {
        out.push(GpuStats::default());
    }
    if let (Some(first), Some(load)) = (out.first_mut(), engine) {
        if first.load.map_or(true, |l| l <= 0.0) {
            first.load = Some(load);
        }
    }
    out
}

#[tauri::command]
pub async fn gpu_stats(app: tauri::AppHandle) -> Vec<GpuStats> {
    tauri::async_runtime::spawn_blocking(move || {
        if cfg!(windows) {
            ensure_started(&app);
        }
        gpus()
    })
    .await
    .unwrap_or_default()
}

pub fn raw_sensors() -> Vec<(String, Vec<(String, String)>)> {
    let snap = SNAP.lock().map(|s| s.clone()).unwrap_or_default();
    let mut groups: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for s in &snap.sensors {
        let Some(v) = s.value else { continue };
        let unit = match s.st.as_str() {
            "Temperature" => " °C",
            "Fan" => " RPM",
            "Power" => " W",
            "Voltage" => " V",
            "Clock" => " MHz",
            "Load" | "Control" | "Level" => " %",
            "SmallData" => " MB",
            "Data" => " GB",
            _ => "",
        };
        groups.entry(format!("{} ({})", s.hw, s.ht)).or_default().push((format!("{} · {}", s.st, s.name), format!("{v:.2}{unit}")));
    }
    groups.into_iter().collect()
}
