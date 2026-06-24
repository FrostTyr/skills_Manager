use crate::models::AppOption;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct SupportedApp {
    key: &'static str,
    label: &'static str,
    kind: &'static str,
    app_name: &'static str,
}

const SUPPORTED_APPS: &[SupportedApp] = &[
    SupportedApp {
        key: "cursor",
        label: "Cursor",
        kind: "editor",
        app_name: "Cursor",
    },
    SupportedApp {
        key: "vscode",
        label: "VS Code",
        kind: "editor",
        app_name: "Visual Studio Code",
    },
    SupportedApp {
        key: "trae",
        label: "Trae",
        kind: "editor",
        app_name: "Trae",
    },
    SupportedApp {
        key: "sublime",
        label: "Sublime Text",
        kind: "editor",
        app_name: "Sublime Text",
    },
    SupportedApp {
        key: "warp",
        label: "Warp",
        kind: "terminal",
        app_name: "Warp",
    },
    SupportedApp {
        key: "ghostty",
        label: "Ghostty",
        kind: "terminal",
        app_name: "Ghostty",
    },
    SupportedApp {
        key: "terminal",
        label: "Terminal",
        kind: "terminal",
        app_name: "Terminal",
    },
    SupportedApp {
        key: "file-manager",
        label: "Finder",
        kind: "fileManager",
        app_name: "Finder",
    },
];

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn find_executable(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var)
        .map(|path| path.join(name))
        .find(|path| path.is_file() || path.is_symlink())
}

pub fn available_apps() -> Vec<AppOption> {
    SUPPORTED_APPS
        .iter()
        .filter(|app| app.kind == "fileManager" || app_is_installed(app.app_name))
        .map(|app| AppOption {
            key: app.key.to_string(),
            label: app.label.to_string(),
            kind: app.kind.to_string(),
        })
        .collect()
}

pub fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    command_success(
        Command::new("open").arg("-R").arg(path),
        "Unable to open Finder",
        "Finder returned a non-zero exit status",
    )
}

pub fn open_in_app(path: &Path, key: &str) -> Result<(), String> {
    let app = SUPPORTED_APPS
        .iter()
        .find(|app| app.key == key)
        .ok_or_else(|| "Unsupported application".to_string())?;

    if app.kind != "fileManager" && !app_is_installed(app.app_name) {
        return Err(format!("{} is not installed", app.label));
    }

    command_success(
        Command::new("open").arg("-a").arg(app.app_name).arg(path),
        &format!("Unable to open {}", app.label),
        &format!("{} returned a non-zero exit status", app.label),
    )
}

fn app_is_installed(app_name: &str) -> bool {
    Command::new("open")
        .arg("-Ra")
        .arg(app_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn command_success(
    command: &mut Command,
    launch_error: &str,
    status_error: &str,
) -> Result<(), String> {
    command
        .status()
        .map_err(|error| format!("{launch_error}: {error}"))?
        .success()
        .then_some(())
        .ok_or_else(|| status_error.to_string())
}
