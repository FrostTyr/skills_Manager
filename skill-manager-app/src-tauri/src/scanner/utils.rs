use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn resolve_real_path(path: &Path, is_symlink: bool) -> (PathBuf, bool) {
    use std::fs;

    if !is_symlink {
        return (
            path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
            false,
        );
    }

    let target = match fs::read_link(path) {
        Ok(target) if target.is_absolute() => target,
        Ok(target) => path.parent().unwrap_or_else(|| Path::new("")).join(target),
        Err(_) => return (path.to_path_buf(), true),
    };

    if target.exists() {
        (target.canonicalize().unwrap_or(target), false)
    } else {
        (target, true)
    }
}

pub fn find_executable(name: &str) -> Option<PathBuf> {
    crate::platform::find_executable(name)
}

pub fn background_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    configure_background_command(&mut command);
    command
}

#[cfg(windows)]
fn configure_background_command(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_background_command(_command: &mut Command) {}

pub fn skill_id(source_agent: &str, path: &Path) -> String {
    format!("{source_agent}:{}", path.to_string_lossy())
}

pub fn category_from_path(root: &Path, skill_path: &Path) -> Option<String> {
    let relative = skill_path.strip_prefix(root).ok()?;
    let mut components = relative.components();
    let category = components.next()?.as_os_str().to_str()?.trim();

    if components.next().is_none() || category.is_empty() {
        None
    } else {
        Some(category.to_string())
    }
}
