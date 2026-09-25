use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub struct OverlayState {
    corner: Mutex<String>,
    custom: Mutex<Option<(i32, i32)>>,
    locked: Mutex<bool>,
}

impl OverlayState {
    pub fn new() -> Self {
        Self { corner: Mutex::new("tl".into()), custom: Mutex::new(None), locked: Mutex::new(true) }
    }
}

#[derive(Clone, Serialize)]
struct StateEvent {
    visible: bool,
    locked: bool,
}

fn window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(w) = app.get_webview_window("overlay") {
        return Ok(w);
    }
    WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html?overlay=1".into()))
        .title("Fairstack Coreview Overlay")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .shadow(false)
        .visible_on_all_workspaces(true)
        .inner_size(280.0, 220.0)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())
}

fn place(app: &AppHandle, win: &WebviewWindow) {
    let state = app.state::<OverlayState>();
    if let Some((x, y)) = *state.custom.lock().unwrap() {
        let _ = win.set_position(PhysicalPosition::new(x, y));
        return;
    }
    let monitor = win.current_monitor().ok().flatten().or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let size = win.outer_size().unwrap_or_default();
    let (origin, area) = (monitor.position(), monitor.size());
    let margin = (24.0 * monitor.scale_factor()) as i32;
    let right = origin.x + area.width as i32 - size.width as i32 - margin;
    let bottom = origin.y + area.height as i32 - size.height as i32 - margin;
    let (x, y) = match state.corner.lock().unwrap().as_str() {
        "tr" => (right, origin.y + margin),
        "bl" => (origin.x + margin, bottom),
        "br" => (right, bottom),
        _ => (origin.x + margin, origin.y + margin),
    };
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn announce(app: &AppHandle) {
    let visible = app.get_webview_window("overlay").and_then(|w| w.is_visible().ok()).unwrap_or(false);
    let locked = *app.state::<OverlayState>().locked.lock().unwrap();
    let _ = app.emit("overlay-state", StateEvent { visible, locked });
}

fn apply_lock(app: &AppHandle, win: &WebviewWindow, locked: bool) {
    *app.state::<OverlayState>().locked.lock().unwrap() = locked;
    let _ = win.set_ignore_cursor_events(locked);
    announce(app);
}

#[tauri::command]
pub async fn overlay_show(app: AppHandle, corner: String, x: Option<i32>, y: Option<i32>, locked: bool) -> Result<(), String> {
    let win = window(&app)?;
    {
        let state = app.state::<OverlayState>();
        *state.corner.lock().unwrap() = corner;
        *state.custom.lock().unwrap() = x.zip(y);
    }
    place(&app, &win);
    win.show().map_err(|e| e.to_string())?;
    let _ = win.set_always_on_top(true);
    apply_lock(&app, &win, locked);
    Ok(())
}

#[tauri::command]
pub fn overlay_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.hide();
    }
    announce(&app);
}

#[tauri::command]
pub fn overlay_lock(app: AppHandle, locked: bool) {
    if let Some(w) = app.get_webview_window("overlay") {
        apply_lock(&app, &w, locked);
    }
}

#[tauri::command]
pub fn overlay_resize(app: AppHandle, width: f64, height: f64) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.set_size(LogicalSize::new(width.max(80.0), height.max(40.0)));
        place(&app, &w);
    }
}

#[tauri::command]
pub fn overlay_set_custom(app: AppHandle, x: i32, y: i32) {
    *app.state::<OverlayState>().custom.lock().unwrap() = Some((x, y));
}

pub fn toggle_visible(app: &AppHandle) {
    let visible = app.get_webview_window("overlay").and_then(|w| w.is_visible().ok()).unwrap_or(false);
    if visible {
        overlay_hide(app.clone());
    } else {
        let (corner, custom, locked) = {
            let s = app.state::<OverlayState>();
            let c = s.corner.lock().unwrap().clone();
            let p = *s.custom.lock().unwrap();
            let l = *s.locked.lock().unwrap();
            (c, p, l)
        };
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = overlay_show(app, corner, custom.map(|p| p.0), custom.map(|p| p.1), locked).await;
        });
    }
}

pub fn toggle_lock(app: &AppHandle) {
    let locked = *app.state::<OverlayState>().locked.lock().unwrap();
    overlay_lock(app.clone(), !locked);
}
