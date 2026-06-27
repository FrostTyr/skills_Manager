use crate::models::AppOption;
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_EXPAND_SZ, REG_SZ,
};

struct SupportedApp {
    key: &'static str,
    label: &'static str,
    kind: &'static str,
    executable_names: &'static [&'static str],
    registry_names: &'static [&'static str],
    common_locations: &'static [&'static str],
}

const SUPPORTED_APPS: &[SupportedApp] = &[
    SupportedApp {
        key: "cursor",
        label: "Cursor",
        kind: "editor",
        executable_names: &["Cursor.exe"],
        registry_names: &["Cursor"],
        common_locations: &[
            "LOCALAPPDATA|Programs\\cursor\\Cursor.exe",
            "DriveRoots|APP\\cursor\\Cursor.exe",
            "DriveRoots|APP\\Cursor\\Cursor.exe",
        ],
    },
    SupportedApp {
        key: "vscode",
        label: "VS Code",
        kind: "editor",
        executable_names: &["code.exe", "Code.exe"],
        registry_names: &["Visual Studio Code", "VS Code"],
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
        registry_names: &["Trae"],
        common_locations: &[
            "LOCALAPPDATA|Programs\\Trae\\Trae.exe",
            "DriveRoots|APP\\Trae\\Trae.exe",
        ],
    },
    SupportedApp {
        key: "sublime",
        label: "Sublime Text",
        kind: "editor",
        executable_names: &["subl.exe", "sublime_text.exe"],
        registry_names: &["Sublime Text"],
        common_locations: &[
            "ProgramFiles|Sublime Text\\sublime_text.exe",
            "DriveRoots|APP\\Sublime Text\\sublime_text.exe",
        ],
    },
    SupportedApp {
        key: "notepadpp",
        label: "Notepad++",
        kind: "editor",
        executable_names: &["notepad++.exe"],
        registry_names: &["Notepad++"],
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
        registry_names: &["Windows Terminal"],
        common_locations: &[],
    },
    SupportedApp {
        key: "powershell",
        label: "PowerShell",
        kind: "terminal",
        executable_names: &["pwsh.exe", "powershell.exe"],
        registry_names: &["PowerShell"],
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
        registry_names: &["File Explorer", "Windows Explorer"],
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
        .or_else(|| query_uninstall_registry(app))
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

fn query_uninstall_registry(app: &SupportedApp) -> Option<PathBuf> {
    const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for access in [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY] {
            if let Some(path) = query_uninstall_registry_view(root, UNINSTALL_KEY, access, app) {
                return Some(path);
            }
        }
    }
    None
}

fn query_uninstall_registry_view(
    root: HKEY,
    key_path: &str,
    access: u32,
    app: &SupportedApp,
) -> Option<PathBuf> {
    let key_path = wide(key_path);
    let mut key: HKEY = std::ptr::null_mut();
    let open_status = unsafe { RegOpenKeyExW(root, key_path.as_ptr(), 0, access, &mut key) };
    if open_status != 0 {
        return None;
    }

    let mut result = None;
    for index in 0..2048 {
        let Some(subkey_name) = enum_registry_subkey(key, index) else {
            break;
        };
        if let Some(path) = query_uninstall_subkey(key, &subkey_name, access, app) {
            result = Some(path);
            break;
        }
    }

    unsafe {
        RegCloseKey(key);
    }
    result
}

fn query_uninstall_subkey(
    parent: HKEY,
    subkey_name: &str,
    access: u32,
    app: &SupportedApp,
) -> Option<PathBuf> {
    let subkey_name = wide(subkey_name);
    let mut subkey: HKEY = std::ptr::null_mut();
    let open_status =
        unsafe { RegOpenKeyExW(parent, subkey_name.as_ptr(), 0, access, &mut subkey) };
    if open_status != 0 {
        return None;
    }

    let result = read_registry_named_string(subkey, "DisplayName")
        .filter(|display_name| registry_display_name_matches(app, display_name))
        .and_then(|_| installed_app_path_from_registry(subkey, app));

    unsafe {
        RegCloseKey(subkey);
    }
    result
}

fn installed_app_path_from_registry(key: HKEY, app: &SupportedApp) -> Option<PathBuf> {
    read_registry_named_string(key, "DisplayIcon")
        .and_then(|value| executable_from_registry_value(&value, app))
        .or_else(|| {
            read_registry_named_string(key, "InstallLocation")
                .and_then(|value| executable_from_install_location(&value, app))
        })
}

fn executable_from_install_location(value: &str, app: &SupportedApp) -> Option<PathBuf> {
    let base = PathBuf::from(expand_environment_path(value.trim().trim_matches('"')));
    app.executable_names.iter().find_map(|name| {
        let candidate = base.join(name);
        candidate
            .is_file()
            .then(|| candidate.canonicalize().unwrap_or(candidate))
    })
}

fn executable_from_registry_value(value: &str, app: &SupportedApp) -> Option<PathBuf> {
    let candidate = registry_executable_path(value)?;
    let path = PathBuf::from(expand_environment_path(&candidate));
    let file_name = path.file_name().and_then(OsStr::to_str)?;
    app.executable_names
        .iter()
        .any(|name| name.eq_ignore_ascii_case(file_name))
        .then_some(())?;
    path.is_file().then(|| path.canonicalize().unwrap_or(path))
}

fn registry_executable_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        return rest.find('"').map(|end| rest[..end].to_string());
    }

    let without_args = trimmed.split(',').next()?.trim();
    let lower = without_args.to_lowercase();
    lower
        .find(".exe")
        .map(|end| without_args[..end + 4].trim().to_string())
        .or_else(|| Some(without_args.to_string()))
}

fn registry_display_name_matches(app: &SupportedApp, display_name: &str) -> bool {
    let display_name = display_name.to_lowercase();
    app.registry_names
        .iter()
        .any(|name| display_name.contains(&name.to_lowercase()))
}

fn expand_environment_path(value: &str) -> String {
    let mut output = String::new();
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        output.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        let Some(end) = rest.find('%') else {
            output.push('%');
            output.push_str(rest);
            return output;
        };
        let name = &rest[..end];
        if let Some(value) = std::env::var_os(name) {
            output.push_str(&value.to_string_lossy());
        } else {
            output.push('%');
            output.push_str(name);
            output.push('%');
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    output
}

fn enum_registry_subkey(key: HKEY, index: u32) -> Option<String> {
    let mut buffer = vec![0u16; 256];
    let mut len = buffer.len() as u32;
    let status = unsafe {
        RegEnumKeyExW(
            key,
            index,
            buffer.as_mut_ptr(),
            &mut len,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        return None;
    }
    buffer.truncate(len as usize);
    Some(String::from_utf16_lossy(&buffer))
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
        if variable == "DriveRoots" {
            return drive_roots().into_iter().find_map(|base| {
                let candidate = base.join(relative);
                candidate
                    .is_file()
                    .then(|| candidate.canonicalize().unwrap_or(candidate))
            });
        }

        let base = std::env::var_os(variable)?;
        let candidate = PathBuf::from(base).join(relative);
        candidate
            .is_file()
            .then(|| candidate.canonicalize().unwrap_or(candidate))
    })
}

fn drive_roots() -> Vec<PathBuf> {
    (b'A'..=b'Z')
        .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
        .filter(|path| path.is_dir())
        .collect()
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

fn read_registry_named_string(key: HKEY, name: &str) -> Option<String> {
    let name = wide(name);
    read_registry_string_value(key, name.as_ptr())
}
fn read_registry_string(key: HKEY) -> Option<String> {
    read_registry_string_value(key, std::ptr::null())
}

fn read_registry_string_value(key: HKEY, value_name: *const u16) -> Option<String> {
    let mut value_type = 0;
    let mut byte_len = 0;
    let first = unsafe {
        RegQueryValueExW(
            key,
            value_name,
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
            value_name,
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
