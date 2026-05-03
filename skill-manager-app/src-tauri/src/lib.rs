mod commands;
mod models;
mod scanner;

use commands::ScannedPaths;
use std::collections::HashSet;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ScannedPaths(Mutex::new(HashSet::new())))
        .invoke_handler(tauri::generate_handler![
            commands::scan_skills,
            commands::reveal_in_finder,
            commands::open_in_editor
        ])
        .run(tauri::generate_context!())
        .expect("error while running Skill Manager")
}
