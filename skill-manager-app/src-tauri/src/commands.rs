use crate::models::{AppOption, ScanResult, SkillFileContent, SkillFileEntry};
use crate::platform;
use crate::scanner;
use crate::services::skill_files;
use crate::state::ScannedPaths;
use tauri::State;

#[tauri::command]
pub async fn scan_skills(state: State<'_, ScannedPaths>) -> Result<ScanResult, String> {
    let result = tauri::async_runtime::spawn_blocking(scanner::scan_skills)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;

    state.replace(result.skills.iter().flat_map(|skill| {
        std::iter::once(skill.path.as_path())
            .chain((skill.path != skill.real_path).then_some(skill.real_path.as_path()))
    }))?;

    Ok(result)
}

#[tauri::command]
pub async fn reveal_in_file_manager(
    path: String,
    state: State<'_, ScannedPaths>,
) -> Result<(), String> {
    platform::reveal_in_file_manager(&state.resolve(&path)?)
}

#[tauri::command]
pub async fn list_skill_files(
    path: String,
    state: State<'_, ScannedPaths>,
) -> Result<Vec<SkillFileEntry>, String> {
    skill_files::list(&state.resolve(&path)?)
}

#[tauri::command]
pub async fn read_skill_file(
    path: String,
    relative_path: String,
    state: State<'_, ScannedPaths>,
) -> Result<SkillFileContent, String> {
    skill_files::read(&state.resolve(&path)?, &relative_path)
}

#[tauri::command]
pub async fn available_apps() -> Vec<AppOption> {
    platform::available_apps()
}

#[tauri::command]
pub async fn open_in_app(
    path: String,
    app: String,
    state: State<'_, ScannedPaths>,
) -> Result<(), String> {
    platform::open_in_app(&state.resolve(&path)?, &app)
}
