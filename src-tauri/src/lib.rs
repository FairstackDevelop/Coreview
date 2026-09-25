mod details;
mod extras;
mod hw;
#[cfg(target_os = "macos")]
mod ioreport;
mod live;
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
        .manage(live::AppState::new())
        .manage(stress::StressState::new())
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
