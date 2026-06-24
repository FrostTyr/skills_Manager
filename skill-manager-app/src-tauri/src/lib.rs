mod commands;
mod models;
mod platform;
mod scanner;
mod services;
mod state;

use state::ScannedPaths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ScannedPaths::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan_skills,
            commands::reveal_in_file_manager,
            commands::list_skill_files,
            commands::read_skill_file,
            commands::available_apps,
            commands::open_in_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running Skill Manager")
}
