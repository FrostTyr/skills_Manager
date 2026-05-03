use crate::models::ScanResult;
use crate::scanner;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use tauri::State;

pub struct ScannedPaths(pub Mutex<HashSet<PathBuf>>);

#[tauri::command]
pub async fn scan_skills(state: State<'_, ScannedPaths>) -> Result<ScanResult, String> {
    let result = tauri::async_runtime::spawn_blocking(scanner::scan_skills)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;

    {
        let mut paths = state.0.lock().unwrap();
        paths.clear();
        for skill in &result.skills {
            paths.insert(skill.path.clone());
            if skill.path != skill.real_path {
                paths.insert(skill.real_path.clone());
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn reveal_in_finder(path: String, state: State<'_, ScannedPaths>) -> Result<(), String> {
    let path = validate_scanned_path(path, &state)?;
    Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|error| format!("Unable to open Finder: {error}"))?
        .success()
        .then_some(())
        .ok_or_else(|| "Finder returned a non-zero exit status".to_string())
}

#[tauri::command]
pub async fn open_in_editor(
    path: String,
    editor: String,
    state: State<'_, ScannedPaths>,
) -> Result<(), String> {
    let path = validate_scanned_path(path, &state)?;
    let editor_name = match editor.as_str() {
        "cursor" => "Cursor",
        "vscode" => "Visual Studio Code",
        _ => return Err("Unsupported editor".to_string()),
    };

    Command::new("open")
        .arg("-a")
        .arg(editor_name)
        .arg(path)
        .status()
        .map_err(|error| format!("Unable to open editor: {error}"))?
        .success()
        .then_some(())
        .ok_or_else(|| format!("{editor_name} returned a non-zero exit status"))
}

fn validate_scanned_path(path: String, state: &State<'_, ScannedPaths>) -> Result<PathBuf, String> {
    let requested = PathBuf::from(&path);

    let canonical = requested
        .canonicalize()
        .map_err(|_| format!("Path does not exist or is not accessible: {path}"))?;

    let paths = state.0.lock().unwrap();
    if !paths.contains(&requested) && !paths.contains(&canonical) {
        return Err(format!(
            "Path was not found in the last scan results: {path}"
        ));
    }

    Ok(canonical)
}
