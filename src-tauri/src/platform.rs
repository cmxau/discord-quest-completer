//! The few OS-specific helpers (kill by name, open a URL / folder, hide console windows). The
//! Windows branches are the original behaviour; macOS and Linux use their standard tools.

use std::path::Path;
use std::process::Command;

/// Windows' CREATE_NO_WINDOW, so no console window flashes when a helper process starts.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A `Command` that never flashes a console window (a no-op off Windows).
pub fn quiet(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, CREATE_NO_WINDOW);
    cmd
}

/// A single file name (no folders): what the frontend may send as an executable name. Rejects
/// separators, `.`/`..`, control characters and the characters Windows forbids in file names.
pub fn validate_file_name(name: &str) -> Result<&str, String> {
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.chars().count() > 255
        || name.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*'));
    if bad { Err(format!("Invalid file name: {name:?}")) } else { Ok(name) }
}

/// Kill every process whose executable file name is `name`.
pub fn kill_by_name(name: &str) -> Result<(), String> {
    let name = validate_file_name(name)?;
    #[cfg(windows)]
    let out = quiet("taskkill").args(["/F", "/IM", name]).output();
    // The fakes are started by full path with `--title`, so match on that (comm is truncated).
    // The name must be a whole path component (`/name --title` or `/name.app/... --title`), so a
    // short name such as "e" cannot match unrelated processes.
    #[cfg(not(windows))]
    let out = {
        let escaped: String = name.chars().flat_map(|c| if "\\.^$|?*+()[]{}".contains(c) { vec!['\\', c] } else { vec![c] }).collect();
        Command::new("pkill").args(["-f", "--", &format!("/{escaped}(/.*)? --title ")]).output()
    };
    let out = out.map_err(|e| format!("Failed to stop the process: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("Failed to stop process: {}", String::from_utf8_lossy(&out.stderr)))
    }
}

/// Hand a URL to the default browser. Callers must have checked it against an allowlist first.
pub fn open_url(url: &str) -> Result<(), String> {
    // No shell is involved, so characters such as `&` in the query string pass through untouched.
    #[cfg(windows)]
    let mut cmd = { let mut c = quiet("rundll32"); c.args(["url.dll,FileProtocolHandler", url]); c };
    #[cfg(target_os = "macos")]
    let mut cmd = { let mut c = Command::new("open"); c.arg(url); c };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = { let mut c = Command::new("xdg-open"); c.arg(url); c };
    cmd.spawn().map(|_| ()).map_err(|e| format!("Failed to open the browser: {e}"))
}

/// Show a file selected in the file manager, or open a folder.
#[cfg(windows)]
pub fn reveal(path: &Path, is_file: bool) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // Explorer wants backslashes; the Steam path from the registry mixes both.
    let p = path.to_string_lossy().replace('/', "\\");
    let mut cmd = Command::new("explorer.exe");
    cmd.raw_arg(if is_file { format!("/select,\"{p}\"") } else { format!("\"{p}\"") });
    // Explorer exits non-zero even on success, so only starting it can fail.
    cmd.spawn().map(|_| ()).map_err(|e| format!("Failed to open Explorer: {e}"))
}

#[cfg(not(windows))]
pub fn reveal(path: &Path, is_file: bool) -> Result<(), String> {
    let mut cmd;
    if cfg!(target_os = "macos") {
        cmd = Command::new("open");
        if is_file { cmd.arg("-R"); }
        cmd.arg(path);
    } else {
        // xdg-open can't select a file, so open its folder.
        cmd = Command::new("xdg-open");
        cmd.arg(if is_file { path.parent().unwrap_or(path) } else { path });
    }
    cmd.spawn().map(|_| ()).map_err(|e| format!("Failed to open the file manager: {e}"))
}

/// Names of all running processes, one per line (`tasklist` CSV on Windows, `ps` elsewhere).
pub fn process_list() -> Result<String, String> {
    #[cfg(windows)]
    let out = quiet("tasklist").args(["/FO", "CSV", "/NH"]).output();
    #[cfg(not(windows))]
    let out = Command::new("ps").args(["-axco", "comm"]).output();
    out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .map_err(|e| format!("Failed to list processes: {e}"))
}

/// "macOS 15.1", "Ubuntu 24.04 LTS" or "Windows 11 (10.0.…)" for pre-filling a bug report.
pub fn os_version() -> String {
    #[cfg(windows)]
    return quiet("cmd").args(["/C", "ver"]).output()
        .map(|out| crate::feedback::parse_windows_version(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_else(|_| "Windows".to_string());
    #[cfg(target_os = "macos")]
    return Command::new("sw_vers").arg("-productVersion").output()
        .map(|o| format!("macOS {}", String::from_utf8_lossy(&o.stdout).trim()))
        .unwrap_or_else(|_| "macOS".to_string());
    #[cfg(all(unix, not(target_os = "macos")))]
    return std::fs::read_to_string("/etc/os-release").ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string())))
        .unwrap_or_else(|| "Linux".to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_cannot_hold_paths_or_odd_characters() {
        for ok in ["game.exe", "roblox.app", "my game.exe", "missed messages..exe", "あ.exe"] {
            assert_eq!(validate_file_name(ok), Ok(ok));
        }
        for bad in ["", ".", "..", "../x", "a/b", "a\\b", "/abs", "C:x", "x.exe:ads", "a\nb", "a\0b", "<x>", &"x".repeat(256)] {
            assert!(validate_file_name(bad).is_err(), "{bad:?}");
            assert!(kill_by_name(bad).is_err(), "{bad:?}");
        }
    }
}
