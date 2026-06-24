use crate::models::AppOption;
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_EXPAND_SZ, REG_SZ,
};

struct SupportedApp {
    key: &'static str,
    label: &'static str,
    kind: &'static str,
    executable_names: &'static [&'static str],
    common_locations: &'static [&'static str],
}

const SUPPORTED_APPS: &[SupportedApp] = &[
    SupportedApp {
        key: "cursor",
        label: "Cursor",
        kind: "editor",
        executable_names: &["Cursor.exe"],
        common_locations: &["LOCALAPPDATA|Programs\\cursor\\Cursor.exe"],
    },
    SupportedApp {
        key: "vscode",
        label: "VS Code",
        kind: "editor",
        executable_names: &["code.exe", "Code.exe"],
        common_locations: &[
            "LOCALAPPDATA|Programs\\Microsoft VS Code\\Code.exe",
            "ProgramFiles|Microsoft VS Code\\Code.exe",
        ],
    },
    SupportedApp {
        key: "trae",
        label: "Trae",
        kind: "editor",
        executable_names: &["Trae.exe"],
        common_locations: &["LOCALAPPDATA|Programs\\Trae\\Trae.exe"],
    },
    SupportedApp {
        key: "sublime",
        label: "Sublime Text",
        kind: "editor",
        executable_names: &["subl.exe", "sublime_text.exe"],
        common_locations: &["ProgramFiles|Sublime Text\\sublime_text.exe"],
    },
    SupportedApp {
        key: "notepadpp",
        label: "Notepad++",
        kind: "editor",
        executable_names: &["notepad++.exe"],
        common_locations: &[
            "ProgramFiles|Notepad++\\notepad++.exe",
            "ProgramFiles(x86)|Notepad++\\notepad++.exe",
        ],
    },
    SupportedApp {
        key: "windows-terminal",
        label: "Windows Terminal",
        kind: "terminal",
        executable_names: &["wt.exe"],
        common_locations: &[],
    },
    SupportedApp {
        key: "powershell",
        label: "PowerShell",
        kind: "terminal",
        executable_names: &["pwsh.exe", "powershell.exe"],
        common_locations: &[
            "ProgramFiles|PowerShell\\7\\pwsh.exe",
            "SystemRoot|System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        ],
    },
    SupportedApp {
        key: "file-manager",
        label: "File Explorer",
        kind: "fileManager",
        executable_names: &["explorer.exe"],
        common_locations: &["SystemRoot|explorer.exe"],
    },
];

pub fn home_dir() -> Option<PathBuf> {
    home_dir_from(
        std::env::var_os("USERPROFILE"),
        std::env::var_os("HOMEDRIVE"),
        std::env::var_os("HOMEPATH"),
    )
}

pub fn find_executable(name: &str) -> Option<PathBuf> {
    search_in_path(name)
        .or_else(|| query_app_paths(name))
        .or_else(|| search_known_common_locations(name))
}

pub fn available_apps() -> Vec<AppOption> {
    SUPPORTED_APPS
        .iter()
        .filter(|app| locate_app(app).is_some())
        .map(|app| AppOption {
            key: app.key.to_string(),
            label: app.label.to_string(),
            kind: app.kind.to_string(),
        })
        .collect()
}

pub fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    let explorer = find_executable("explorer.exe").unwrap_or_else(|| PathBuf::from("explorer.exe"));
    let mut selection = OsString::from("/select,");
    selection.push(path.as_os_str());
    command_success(
        Command::new(explorer).arg(selection),
        "Unable to open File Explorer",
        "File Explorer returned a non-zero exit status",
    )
}

pub fn open_in_app(path: &Path, key: &str) -> Result<(), String> {
    let app = SUPPORTED_APPS
        .iter()
        .find(|app| app.key == key)
        .ok_or_else(|| "Unsupported application".to_string())?;
    let executable = locate_app(app).ok_or_else(|| format!("{} is not installed", app.label))?;
    let mut command = Command::new(executable);

    match app.key {
        "file-manager" => {
            command.arg(path);
        }
        "windows-terminal" => {
            command.arg("-d").arg(path);
        }
        "powershell" => {
            command.current_dir(path).arg("-NoExit");
        }
        _ => {
            command.arg(path);
        }
    }

    command_success(
        &mut command,
        &format!("Unable to open {}", app.label),
        &format!("{} returned a non-zero exit status", app.label),
    )
}

pub(crate) fn windows_path_key(path: &Path) -> String {
    let value = path.to_string_lossy().replace('/', "\\");
    let value = value
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .or_else(|| value.strip_prefix(r"\\?\").map(ToString::to_string))
        .unwrap_or(value);
    value.trim_end_matches('\\').to_lowercase()
}

fn locate_app(app: &SupportedApp) -> Option<PathBuf> {
    app.executable_names
        .iter()
        .find_map(|name| search_in_path(name))
        .or_else(|| {
            app.executable_names
                .iter()
                .find_map(|name| query_app_paths(name))
        })
        .or_else(|| search_common_locations(app.common_locations))
}

fn search_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let path_ext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    search_in_path_dirs(name, &path_ext, std::env::split_paths(&path))
}

fn search_in_path_dirs(
    name: &str,
    path_ext: &str,
    directories: impl IntoIterator<Item = PathBuf>,
) -> Option<PathBuf> {
    let candidates = executable_names(name, path_ext);
    for dir in directories {
        for candidate_name in &candidates {
            let candidate = dir.join(candidate_name);
            if candidate.is_file() {
                return Some(candidate.canonicalize().unwrap_or(candidate));
            }
        }
    }
    None
}

fn executable_names(name: &str, path_ext: &str) -> Vec<String> {
    if Path::new(name).extension().is_some() {
        return vec![name.to_string()];
    }

    path_ext
        .split(';')
        .filter(|extension| !extension.is_empty())
        .map(|extension| {
            if extension.starts_with('.') {
                format!("{name}{extension}")
            } else {
                format!("{name}.{extension}")
            }
        })
        .collect()
}

fn query_app_paths(name: &str) -> Option<PathBuf> {
    let path_ext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    for candidate in executable_names(name, &path_ext) {
        let key_path = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{candidate}");
        for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            for access in [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY] {
                if let Some(path) = read_registry_default(root, &key_path, access) {
                    if path.is_file() {
                        return Some(path.canonicalize().unwrap_or(path));
                    }
                }
            }
        }
    }
    None
}

fn search_known_common_locations(name: &str) -> Option<PathBuf> {
    SUPPORTED_APPS
        .iter()
        .filter(|app| {
            app.executable_names
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(name))
        })
        .find_map(|app| search_common_locations(app.common_locations))
}

fn search_common_locations(locations: &[&str]) -> Option<PathBuf> {
    locations.iter().find_map(|location| {
        let (variable, relative) = location.split_once('|')?;
        let base = std::env::var_os(variable)?;
        let candidate = PathBuf::from(base).join(relative);
        candidate
            .is_file()
            .then(|| candidate.canonicalize().unwrap_or(candidate))
    })
}

fn read_registry_default(root: HKEY, key_path: &str, access: u32) -> Option<PathBuf> {
    let key_path = wide(key_path);
    let mut key: HKEY = std::ptr::null_mut();
    let open_status = unsafe { RegOpenKeyExW(root, key_path.as_ptr(), 0, access, &mut key) };
    if open_status != 0 {
        return None;
    }

    let result = read_registry_string(key);
    unsafe {
        RegCloseKey(key);
    }
    result.map(PathBuf::from)
}

fn read_registry_string(key: HKEY) -> Option<String> {
    let mut value_type = 0;
    let mut byte_len = 0;
    let first = unsafe {
        RegQueryValueExW(
            key,
            std::ptr::null(),
            std::ptr::null_mut(),
            &mut value_type,
            std::ptr::null_mut(),
            &mut byte_len,
        )
    };
    if first != 0 || !matches!(value_type, REG_SZ | REG_EXPAND_SZ) || byte_len < 2 {
        return None;
    }

    let mut buffer = vec![0u16; byte_len as usize / 2];
    let second = unsafe {
        RegQueryValueExW(
            key,
            std::ptr::null(),
            std::ptr::null_mut(),
            &mut value_type,
            buffer.as_mut_ptr().cast(),
            &mut byte_len,
        )
    };
    if second != 0 {
        return None;
    }

    while buffer.last() == Some(&0) {
        buffer.pop();
    }
    Some(String::from_utf16_lossy(&buffer))
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

fn home_dir_from(
    user_profile: Option<OsString>,
    home_drive: Option<OsString>,
    home_path: Option<OsString>,
) -> Option<PathBuf> {
    if let Some(user_profile) = user_profile {
        return Some(PathBuf::from(user_profile));
    }

    let mut combined = home_drive?;
    combined.push(home_path?);
    Some(PathBuf::from(combined))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_names_respect_pathext_order() {
        assert_eq!(
            executable_names("openclaw", ".CMD;.EXE;.BAT"),
            vec!["openclaw.CMD", "openclaw.EXE", "openclaw.BAT"]
        );
        assert_eq!(
            executable_names("openclaw.exe", ".CMD;.EXE"),
            vec!["openclaw.exe"]
        );
    }

    #[test]
    fn path_search_uses_pathext_priority() {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(temp.path().join("openclaw.exe"), "").unwrap();
        std::fs::write(temp.path().join("openclaw.cmd"), "").unwrap();
        std::fs::write(temp.path().join("openclaw.bat"), "").unwrap();

        let found = search_in_path_dirs(
            "openclaw",
            ".CMD;.EXE;.BAT",
            vec![temp.path().to_path_buf()],
        )
        .unwrap();
        assert_eq!(
            found.file_name().and_then(OsStr::to_str),
            Some("openclaw.cmd")
        );
    }

    #[test]
    fn user_profile_precedes_legacy_home_parts() {
        assert_eq!(
            home_dir_from(
                Some(OsString::from(r"C:\Users\Primary")),
                Some(OsString::from("D:")),
                Some(OsString::from(r"\Users\Fallback")),
            ),
            Some(PathBuf::from(r"C:\Users\Primary"))
        );
    }

    #[test]
    fn windows_path_keys_ignore_case_and_long_path_prefix() {
        assert_eq!(
            windows_path_key(Path::new(r"\\?\C:\Users\Test\Skills")),
            windows_path_key(Path::new(r"c:\users\test\skills"))
        );
    }
}
