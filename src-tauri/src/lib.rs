#[cfg(windows)]
mod cpuperf;
mod details;
mod extras;
mod fps;
mod history;
mod hw;
#[cfg(target_os = "macos")]
mod ioreport;
mod live;
mod overlay;
mod power;
mod smc;
mod startup;
mod stress;
mod util;
mod winsensors;

fn log_line(text: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("fairstack-coreview.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{text}");
    }
}

fn install_crash_log() {
    std::panic::set_hook(Box::new(|info| {
        log_line(&format!("PANIC: {info}\n{}", std::backtrace::Backtrace::force_capture()));
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_crash_log();
    log_line(&format!("start {} {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS));
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let mods = Modifiers::CONTROL | Modifiers::SHIFT;
                    if shortcut.matches(mods, Code::F9) {
                        overlay::toggle_visible(app);
                    } else if shortcut.matches(mods, Code::F10) {
                        overlay::toggle_lock(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
            let mods = Modifiers::CONTROL | Modifiers::SHIFT;
            for code in [Code::F9, Code::F10] {
                if let Err(e) = app.global_shortcut().register(Shortcut::new(Some(mods), code)) {
                    log_line(&format!("shortcut error: {e}"));
                }
            }
            Ok(())
        })
        .manage(live::AppState::new())
        .manage(stress::StressState::new())
        .manage(overlay::OverlayState::new())
        .invoke_handler(tauri::generate_handler![
            hw::hardware_info,
            live::live_stats,
            live::processes,
            live::kill_process,
            startup::startup_items,
            startup::startup_set_enabled,
            startup::startup_remove,
            startup::startup_add,
            power::power_stats,
            smc::smc_sensors,
            winsensors::sensor_status,
            winsensors::install_sensor_driver,
            stress::stress_start,
            stress::stress_stop,
            stress::stress_status,
            history::history_append,
            history::history_query,
            history::history_csv,
            history::history_prune,
            history::history_clear,
            overlay::overlay_show,
            overlay::overlay_hide,
            overlay::overlay_lock,
            overlay::overlay_resize,
            overlay::overlay_set_custom,
            fps::fps_enable,
            fps::fps_status,
            fps::fps_stats,
            details::detail_categories,
            details::detail_data,
            extras::connections,
            extras::save_snapshot,
            extras::list_snapshots,
            extras::load_snapshot,
            extras::delete_snapshot,
            extras::save_text_file,
            extras::set_window_effect,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| log_line(&format!("ERROR: {e}")));
}
