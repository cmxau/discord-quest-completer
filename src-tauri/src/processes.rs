//! Dummy game processes we started, so the UI can tell when one exits on its own.

use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::path::Path;
use std::process::Child;
use std::sync::{Mutex, MutexGuard};

// Keyed by lowercase exe file name. Keeping the handles lets us notice when a process exits on its
// own (closed from its window, killed in Task Manager, ...).
static CHILDREN: OnceCell<Mutex<HashMap<String, Vec<Child>>>> = OnceCell::new();

fn children() -> MutexGuard<'static, HashMap<String, Vec<Child>>> {
    CHILDREN
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Remember a started process under its exe file name (a full path is reduced to the file name).
pub fn register_child(exe_name: &str, child: Child) {
    let key = Path::new(exe_name)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| exe_name.to_lowercase());
    children().entry(key).or_default().push(child);
}

/// File names (lowercase) of the dummy processes that are still running. Exited ones are reaped.
#[tauri::command]
pub fn running_exes() -> Vec<String> {
    let mut map = children();
    let mut running = Vec::new();
    map.retain(|name, list| {
        list.retain_mut(|child| matches!(child.try_wait(), Ok(None)));
        if list.is_empty() {
            false
        } else {
            running.push(name.clone());
            true
        }
    });
    running
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn sleeper(seconds: u32) -> Child {
        Command::new("ping")
            .args(["-n", &(seconds + 1).to_string(), "127.0.0.1"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap()
    }

    #[test]
    fn running_exes_follows_real_process_lifetimes() {
        register_child(r"C:\games\Long Running.EXE", sleeper(30));
        register_child("short.exe", Command::new("cmd").args(["/C", "exit", "0"]).spawn().unwrap());

        // The short-lived process exits on its own; the long one is reported (names are lowercase file names).
        let mut running = Vec::new();
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            running = running_exes();
            if !running.contains(&"short.exe".to_string()) {
                break;
            }
        }
        assert_eq!(running, vec!["long running.exe".to_string()]);

        // Kill the long one the way the UI's Stop button does and it disappears too.
        let _ = Command::new("taskkill").args(["/F", "/IM", "ping.exe"]).output();
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if running_exes().is_empty() {
                return;
            }
        }
        panic!("killed process still reported as running: {:?}", running_exes());
    }
}
