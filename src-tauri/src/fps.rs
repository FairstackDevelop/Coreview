#![allow(dead_code)]

use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Manager;

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FpsStats {
    fps: f64,
    low1: f64,
    frame_ms: f64,
    app: String,
    pid: u32,
    history: Vec<f32>,
}

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FpsStatus {
    supported: bool,
    running: bool,
    available: bool,
    error: String,
}

#[derive(Default)]
struct Inner {
    frames: HashMap<u32, VecDeque<(Instant, f32)>>,
    names: HashMap<u32, String>,
    error: String,
    running: bool,
    available: bool,
    generation: u64,
    child: Option<Arc<Mutex<Child>>>,
}

static STATE: Mutex<Option<Inner>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Inner) -> R) -> R {
    let mut guard = STATE.lock().unwrap();
    f(guard.get_or_insert_with(Inner::default))
}

const EXE: &str = "PresentMon.exe";
const IGNORED: [&str; 8] = ["dwm.exe", "explorer.exe", "shellexperiencehost.exe", "searchhost.exe", "startmenuexperiencehost.exe", "textinputhost.exe", "applicationframehost.exe", "widgets.exe"];

fn locate(app: &tauri::AppHandle) -> Option<PathBuf> {
    let mut list = Vec::new();
    if let Ok(r) = app.path().resource_dir() {
        list.push(r.join("sensors").join(EXE));
        list.push(r.join(EXE));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        list.push(dir.join("sensors").join(EXE));
        list.push(dir.join("../../../sensors-helper/out").join(EXE));
        list.push(dir.join("../../../../sensors-helper/out").join(EXE));
    }
    list.into_iter().find(|p| p.exists())
}

#[cfg(windows)]
fn foreground_pid() -> Option<u32> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        (pid != 0).then_some(pid)
    }
}

#[cfg(not(windows))]
fn foreground_pid() -> Option<u32> {
    None
}

fn attempts() -> Vec<Vec<&'static str>> {
    vec![
        vec!["--output_stdout", "--no_console_stats", "--stop_existing_session"],
        vec!["--output_stdout", "--no_console_stats"],
        vec!["-output_stdout", "-no_top", "-stop_existing_session"],
    ]
}

fn spawn(exe: &PathBuf, args: &[&str]) -> Option<Child> {
    let mut cmd = Command::new(exe);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.spawn().ok()
}

fn ingest(line: &str, columns: &mut Option<(usize, usize, usize)>) {
    let cells: Vec<&str> = line.trim().split(',').collect();
    if cells.first().map(|c| c.trim_start_matches('\u{feff}')) == Some("Application") {
        let find = |names: &[&str]| cells.iter().position(|c| names.iter().any(|n| n.eq_ignore_ascii_case(c)));
        *columns = match (find(&["Application"]), find(&["ProcessID"]), find(&["FrameTime", "MsBetweenPresents"])) {
            (Some(a), Some(p), Some(f)) => Some((a, p, f)),
            _ => None,
        };
        return;
    }
    let Some((a, p, f)) = *columns else { return };
    let (Some(app), Some(pid), Some(ms)) = (cells.get(a), cells.get(p).and_then(|v| v.parse::<u32>().ok()), cells.get(f).and_then(|v| v.parse::<f32>().ok())) else { return };
    if !(0.05..2000.0).contains(&ms) {
        return;
    }
    with(|s| {
        s.available = true;
        s.names.insert(pid, app.to_string());
        let q = s.frames.entry(pid).or_default();
        q.push_back((Instant::now(), ms));
        while q.len() > 2000 || q.front().map(|(t, _)| t.elapsed() > Duration::from_secs(8)).unwrap_or(false) {
            q.pop_front();
        }
    });
}

fn run(exe: PathBuf, generation: u64) {
    for args in attempts() {
        if with(|s| s.generation) != generation {
            return;
        }
        let Some(mut child) = spawn(&exe, &args) else { continue };
        let Some(out) = child.stdout.take() else { continue };
        let shared = Arc::new(Mutex::new(child));
        with(|s| s.child = Some(shared.clone()));
        let started = Instant::now();
        let mut columns = None;
        let mut got_any = false;
        for line in BufReader::new(out).lines() {
            let Ok(line) = line else { break };
            got_any = true;
            ingest(&line, &mut columns);
            if with(|s| s.generation) != generation {
                break;
            }
        }
        let _ = shared.lock().unwrap().kill();
        let _ = shared.lock().unwrap().wait();
        if with(|s| s.generation) != generation {
            return;
        }
        if got_any || started.elapsed() > Duration::from_secs(5) {
            break;
        }
    }
    with(|s| {
        if s.generation == generation {
            s.running = false;
            if !s.available {
                s.error = "PresentMon stopped".into();
            }
        }
    });
}

#[tauri::command]
pub fn fps_enable(app: tauri::AppHandle, enabled: bool) -> FpsStatus {
    if !cfg!(windows) {
        return FpsStatus::default();
    }
    if enabled {
        let start = with(|s| {
            if s.running {
                return None;
            }
            s.running = true;
            s.error.clear();
            s.generation += 1;
            Some(s.generation)
        });
        if let Some(generation) = start {
            match locate(&app) {
                Some(exe) => {
                    std::thread::spawn(move || run(exe, generation));
                }
                None => with(|s| {
                    s.running = false;
                    s.error = "PresentMon is not bundled".into();
                }),
            }
        }
    } else {
        with(|s| {
            s.generation += 1;
            s.running = false;
            s.frames.clear();
            if let Some(child) = s.child.take() {
                let _ = child.lock().unwrap().kill();
            }
        });
    }
    fps_status()
}

#[tauri::command]
pub fn fps_status() -> FpsStatus {
    with(|s| FpsStatus { supported: cfg!(windows), running: s.running, available: s.available, error: s.error.clone() })
}

#[tauri::command]
pub fn fps_stats() -> Option<FpsStats> {
    with(|s| {
        let now = Instant::now();
        let recent = |pid: &u32, secs: f64| s.frames.get(pid).map(|q| q.iter().filter(|(t, _)| now.duration_since(*t).as_secs_f64() <= secs).count()).unwrap_or(0);
        let active: Vec<u32> = s.frames.keys().copied().filter(|p| recent(p, 2.0) >= 2).collect();
        let pid = foreground_pid()
            .filter(|p| active.contains(p))
            .or_else(|| active.iter().copied().filter(|p| !IGNORED.contains(&s.names.get(p).map(|n| n.to_lowercase()).unwrap_or_default().as_str())).max_by_key(|p| recent(p, 2.0)))?;
        let queue = s.frames.get(&pid)?;
        let fps = recent(&pid, 1.0) as f64;
        let mut window: Vec<f32> = queue.iter().filter(|(t, _)| now.duration_since(*t).as_secs_f64() <= 5.0).map(|(_, ms)| *ms).collect();
        window.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let worst = ((window.len() as f64 * 0.01).ceil() as usize).max(1).min(window.len());
        let low1 = if window.is_empty() { 0.0 } else { 1000.0 / (window[..worst].iter().sum::<f32>() / worst as f32) as f64 };
        Some(FpsStats {
            fps,
            low1,
            frame_ms: queue.back().map(|(_, ms)| *ms as f64).unwrap_or(0.0),
            app: s.names.get(&pid).cloned().unwrap_or_default(),
            pid,
            history: queue.iter().rev().take(90).map(|(_, ms)| *ms).collect::<Vec<_>>().into_iter().rev().collect(),
        })
    })
}
