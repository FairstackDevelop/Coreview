use crate::live::AppState;
use crate::util::*;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use sysinfo::{Pid, ProcessesToUpdate};
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    process: String,
    pid: u32,
    local: String,
    remote: String,
    state: String,
}

#[tauri::command]
pub async fn connections(state: tauri::State<'_, AppState>) -> Result<Vec<Connection>, String> {
    let mut out = Vec::new();
    if cfg!(windows) {
        let text = tauri::async_runtime::spawn_blocking(|| run("netstat", &["-ano", "-p", "TCP"]).unwrap_or_default())
            .await
            .map_err(|e| e.to_string())?;
        let mut sys = state.get().sys.lock().unwrap();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        for line in text.lines() {
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() == 5 && f[0] == "TCP" {
                let pid: u32 = f[4].parse().unwrap_or(0);
                out.push(Connection {
                    process: sys
                        .process(Pid::from_u32(pid))
                        .map(|p| p.name().to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    pid,
                    local: f[1].into(),
                    remote: f[2].into(),
                    state: f[3].into(),
                });
            }
        }
    } else {
        let text = tauri::async_runtime::spawn_blocking(|| {
            run("lsof", &["-iTCP", "-nP", "-sTCP:ESTABLISHED,LISTEN"]).unwrap_or_default()
        })
        .await
        .map_err(|e| e.to_string())?;
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() < 10 {
                continue;
            }
            let (local, remote) = f[8].split_once("->").unwrap_or((f[8], "*"));
            out.push(Connection {
                process: f[0].replace("\\x20", " "),
                pid: f[1].parse().unwrap_or(0),
                local: local.into(),
                remote: remote.into(),
                state: f[9].trim_matches(|c| c == '(' || c == ')').into(),
            });
        }
    }
    out.truncate(500);
    Ok(out)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMeta {
    id: String,
    name: String,
    created: u64,
}

fn snap_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("snapshots");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn valid_id(id: &str) -> Result<(), String> {
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err("invalid id".into())
    }
}

#[tauri::command]
pub fn save_snapshot(app: tauri::AppHandle, name: String, data: String) -> Result<SnapshotMeta, String> {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    let payload: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    let body = serde_json::json!({ "name": name, "created": created, "data": payload });
    let id = created.to_string();
    fs::write(snap_dir(&app)?.join(format!("{id}.json")), body.to_string()).map_err(|e| e.to_string())?;
    Ok(SnapshotMeta { id, name, created })
}

#[tauri::command]
pub fn list_snapshots(app: tauri::AppHandle) -> Result<Vec<SnapshotMeta>, String> {
    let mut list = Vec::new();
    for e in fs::read_dir(snap_dir(&app)?).map_err(|e| e.to_string())?.flatten() {
        let Ok(text) = fs::read_to_string(e.path()) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        list.push(SnapshotMeta {
            id: e.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            name: s(&v, "name"),
            created: n(&v, "created"),
        });
    }
    list.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(list)
}

#[tauri::command]
pub fn load_snapshot(app: tauri::AppHandle, id: String) -> Result<String, String> {
    valid_id(&id)?;
    let text = fs::read_to_string(snap_dir(&app)?.join(format!("{id}.json"))).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(v.get("data").cloned().unwrap_or_default().to_string())
}

#[tauri::command]
pub fn delete_snapshot(app: tauri::AppHandle, id: String) -> Result<(), String> {
    valid_id(&id)?;
    fs::remove_file(snap_dir(&app)?.join(format!("{id}.json"))).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_window_effect(window: tauri::WebviewWindow, effect: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::*;
        match effect.as_str() {
            "none" => {
                clear_vibrancy(&window).map_err(|e| e.to_string())?;
            }
            _ => apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None).map_err(|e| e.to_string())?,
        }
    }
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::*;
        let _ = clear_mica(&window);
        let _ = clear_acrylic(&window);
        match effect.as_str() {
            "none" => {}
            "mica" => apply_mica(&window, None).map_err(|e| e.to_string())?,
            _ => apply_acrylic(&window, Some((18, 18, 24, 110))).map_err(|e| e.to_string())?,
        }
    }
    let _ = (&window, &effect);
    Ok(())
}
