mod details;
mod extras;
mod hw;
mod live;
mod power;
mod startup;
mod stress;
mod util;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
        .expect("error while running tauri application");
}
