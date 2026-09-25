use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tauri::Manager;

const DAY: u64 = 86_400_000;

fn dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let d = app.path().app_data_dir().map_err(|e| e.to_string())?.join("history");
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

#[tauri::command]
pub async fn history_append(app: tauri::AppHandle, sample: Value) -> Result<(), String> {
    let t = sample.get("t").and_then(|v| v.as_u64()).ok_or("missing timestamp")?;
    let path = dir(&app)?.join(format!("d{}.jsonl", t / DAY));
    let mut file = fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
    writeln!(file, "{sample}").map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct HistoryData {
    points: Vec<Value>,
    marks: Vec<Value>,
}

fn read_range(app: &tauri::AppHandle, from: u64, to: u64) -> Result<(Vec<Value>, Vec<Value>), String> {
    let base = dir(app)?;
    let (mut rows, mut marks) = (Vec::new(), Vec::new());
    for day in (from / DAY)..=(to / DAY) {
        let Ok(text) = fs::read_to_string(base.join(format!("d{day}.jsonl"))) else { continue };
        for line in text.lines() {
            let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
            let Some(t) = v.get("t").and_then(|x| x.as_u64()) else { continue };
            if t < from || t > to {
                continue;
            }
            if v.get("mark").is_some() {
                marks.push(v);
            } else {
                rows.push(v);
            }
        }
    }
    rows.sort_by_key(|v| v.get("t").and_then(|x| x.as_u64()).unwrap_or(0));
    Ok((rows, marks))
}

fn downsample(rows: Vec<Value>, from: u64, to: u64, max_points: usize) -> Vec<Value> {
    if rows.len() <= max_points || max_points == 0 {
        return rows;
    }
    let bucket = ((to - from) as f64 / max_points as f64).max(1.0);
    let mut groups: BTreeMap<u64, BTreeMap<String, (f64, u32)>> = BTreeMap::new();
    for row in &rows {
        let Some(t) = row.get("t").and_then(|x| x.as_u64()) else { continue };
        let index = ((t - from) as f64 / bucket) as u64;
        let group = groups.entry(index).or_default();
        if let Some(map) = row.as_object() {
            for (k, v) in map {
                if k == "t" {
                    continue;
                }
                if let Some(n) = v.as_f64() {
                    let e = group.entry(k.clone()).or_insert((0.0, 0));
                    e.0 += n;
                    e.1 += 1;
                }
            }
        }
    }
    groups
        .into_iter()
        .map(|(index, fields)| {
            let mut m = Map::new();
            m.insert("t".into(), Value::from((from as f64 + (index as f64 + 0.5) * bucket) as u64));
            for (k, (sum, n)) in fields {
                m.insert(k, Value::from((sum / n as f64 * 100.0).round() / 100.0));
            }
            Value::Object(m)
        })
        .collect()
}

#[tauri::command]
pub async fn history_query(app: tauri::AppHandle, from: u64, to: u64, max_points: usize) -> Result<HistoryData, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (rows, marks) = read_range(&app, from, to)?;
        Ok(HistoryData { points: downsample(rows, from, to, max_points), marks })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn history_csv(app: tauri::AppHandle, from: u64, to: u64) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (rows, _) = read_range(&app, from, to)?;
        let mut columns: Vec<String> = Vec::new();
        for row in &rows {
            if let Some(map) = row.as_object() {
                for k in map.keys() {
                    if !columns.contains(k) {
                        columns.push(k.clone());
                    }
                }
            }
        }
        columns.sort_by_key(|c| (c != "t", c.clone()));
        let mut out = columns.join(",");
        out.push('\n');
        for row in &rows {
            let cells: Vec<String> = columns.iter().map(|c| row.get(c).map(|v| v.to_string()).unwrap_or_default()).collect();
            out.push_str(&cells.join(","));
            out.push('\n');
        }
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn history_prune(app: tauri::AppHandle, keep_days: u64, now: u64) -> Result<(), String> {
    let base = dir(&app)?;
    let oldest = (now / DAY).saturating_sub(keep_days);
    for entry in fs::read_dir(base).map_err(|e| e.to_string())?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(day) = name.strip_prefix('d').and_then(|s| s.strip_suffix(".jsonl")).and_then(|s| s.parse::<u64>().ok()) {
            if day < oldest {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn history_clear(app: tauri::AppHandle) -> Result<(), String> {
    for entry in fs::read_dir(dir(&app)?).map_err(|e| e.to_string())?.flatten() {
        let _ = fs::remove_file(entry.path());
    }
    Ok(())
}
