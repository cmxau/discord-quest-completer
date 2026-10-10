//! App-level helpers behind the Settings page: checking that Discord is running, looking for a
//! newer release, starting with Windows, and the tray icon / exit behaviour.

use crate::feedback::{open_in_browser, CREATE_NO_WINDOW};
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

// ---------------------------------------------------------------------------------------------
// Behaviour switches the UI keeps in sync (the settings themselves live in the webview)
// ---------------------------------------------------------------------------------------------

/// The window's close button hides the window to the tray instead of quitting.
pub static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(false);
/// Quitting stops the dummy games (and, unless Steam entries are kept, removes the ones they made).
pub static STOP_ON_EXIT: AtomicBool = AtomicBool::new(true);
/// Steam entries stay in the Steam library after the game stops, until the user removes them.
pub static KEEP_STEAM_ENTRIES: AtomicBool = AtomicBool::new(false);

/// The settings the backend needs. They are saved to a small file because the startup cleanup of
/// leftover Steam entries runs before the page has loaded and could tell it.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(default)]
pub struct Behavior {
    pub close_to_tray: bool,
    pub stop_on_exit: bool,
    pub keep_steam_entries: bool,
}

impl Default for Behavior {
    fn default() -> Self {
        Behavior { close_to_tray: false, stop_on_exit: true, keep_steam_entries: false }
    }
}

/// Read saved behavior; anything missing or unreadable falls back to the defaults.
pub fn parse_behavior(text: &str) -> Behavior {
    serde_json::from_str(text).unwrap_or_default()
}

fn behavior_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join("behavior.json"))
}

fn apply_behavior(behavior: &Behavior) {
    CLOSE_TO_TRAY.store(behavior.close_to_tray, Ordering::Relaxed);
    STOP_ON_EXIT.store(behavior.stop_on_exit, Ordering::Relaxed);
    KEEP_STEAM_ENTRIES.store(behavior.keep_steam_entries, Ordering::Relaxed);
}

/// At startup: restore what the user chose last time.
pub fn load_behavior(app: &AppHandle) {
    if let Some(text) = behavior_path(app).and_then(|p| std::fs::read_to_string(p).ok()) {
        apply_behavior(&parse_behavior(&text));
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_behavior(app: AppHandle, close_to_tray: bool, stop_on_exit: bool, keep_steam_entries: bool) {
    let behavior = Behavior { close_to_tray, stop_on_exit, keep_steam_entries };
    apply_behavior(&behavior);
    if let Some(path) = behavior_path(&app) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string(&behavior) {
            let _ = std::fs::write(path, text);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Tray icon and window
// ---------------------------------------------------------------------------------------------

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// The tray icon is always there; it is how a hidden window comes back and how the app quits.
pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Quest Completer", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut tray = TrayIconBuilder::new()
        .tooltip("Discord Quest Completer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Started with `--minimized` (by "Start with Windows"): stay in the tray until opened.
pub fn hide_window_if_started_minimized(app: &tauri::App) {
    if std::env::args().any(|arg| arg == "--minimized") {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Is Discord running?
// ---------------------------------------------------------------------------------------------

#[derive(Serialize, Debug, Default, PartialEq)]
pub struct DiscordStatus {
    pub running: bool,
    /// Which Discord apps were found, e.g. ["Discord"] or ["Discord", "Discord PTB"].
    pub clients: Vec<String>,
}

const DISCORD_CLIENTS: [(&str, &str); 3] = [
    ("discord.exe", "Discord"),
    ("discordptb.exe", "Discord PTB"),
    ("discordcanary.exe", "Discord Canary"),
];

/// Find Discord in the output of `tasklist /FO CSV /NH` (one `"Image.exe","pid",...` line per process).
pub fn parse_discord_status(tasklist_csv: &str) -> DiscordStatus {
    let mut clients: Vec<String> = Vec::new();
    for line in tasklist_csv.lines() {
        let image = line.trim().trim_start_matches('"').split('"').next().unwrap_or("").to_lowercase();
        if let Some((_, name)) = DISCORD_CLIENTS.iter().find(|(exe, _)| *exe == image) {
            if !clients.iter().any(|c| c == name) {
                clients.push(name.to_string());
            }
        }
    }
    DiscordStatus { running: !clients.is_empty(), clients }
}

#[tauri::command]
pub async fn discord_status() -> Result<DiscordStatus, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let out = Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to list processes: {}", e))?;
        Ok(parse_discord_status(&String::from_utf8_lossy(&out.stdout)))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------------------------
// Update check
// ---------------------------------------------------------------------------------------------

const REPO_URL: &str = "https://github.com/cmxau/discord-quest-completer";
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/cmxau/discord-quest-completer/releases/latest";

#[derive(Serialize, Debug, Default, PartialEq)]
pub struct UpdateInfo {
    pub current: String,
    /// The newest published release, e.g. "26.10.2"; none if there is no release yet.
    pub latest: Option<String>,
    pub update_available: bool,
    /// The release's page on GitHub.
    pub url: Option<String>,
}

/// "v26.10.1" -> [26, 10, 1]. None for anything that isn't dot-separated numbers.
pub fn parse_version(text: &str) -> Option<Vec<u64>> {
    let parts: Vec<u64> = text
        .trim()
        .trim_start_matches(['v', 'V'])
        .split('.')
        .map(|part| part.parse().ok())
        .collect::<Option<_>>()?;
    (!parts.is_empty()).then_some(parts)
}

/// Is `latest` a higher version than `current`? Unreadable versions are never "newer".
pub fn is_newer(latest: &str, current: &str) -> bool {
    let (Some(mut a), Some(mut b)) = (parse_version(latest), parse_version(current)) else {
        return false;
    };
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    a > b
}

/// Read the tag and page of a release out of GitHub's "latest release" response.
pub fn parse_release(json: &serde_json::Value) -> Option<(String, String)> {
    let tag = json["tag_name"].as_str()?.trim().to_string();
    parse_version(&tag)?;
    let url = json["html_url"].as_str().map(str::to_string).unwrap_or_else(|| format!("{REPO_URL}/releases/tag/{tag}"));
    Some((tag.trim_start_matches(['v', 'V']).to_string(), url))
}

#[tauri::command]
pub async fn check_for_update() -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let client = tauri_plugin_http::reqwest::Client::builder()
        .user_agent(format!("discord-quest-completer/{current}"))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get(LATEST_RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Couldn't reach GitHub: {e}"))?;

    // No published release yet.
    if res.status() == tauri_plugin_http::reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateInfo { current, ..Default::default() });
    }
    let text = res
        .error_for_status()
        .map_err(|e| format!("GitHub answered with an error: {e}"))?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    Ok(match parse_release(&json) {
        Some((latest, url)) => UpdateInfo { update_available: is_newer(&latest, &current), current, latest: Some(latest), url: Some(url) },
        None => UpdateInfo { current, ..Default::default() },
    })
}

/// Only this project's releases page (or one release of it) may be opened from the app.
pub fn is_allowed_release_url(url: &str) -> bool {
    let releases = format!("{REPO_URL}/releases");
    let Some(rest) = url.strip_prefix(&releases) else {
        return false;
    };
    match rest {
        "" | "/latest" => true,
        _ => rest
            .strip_prefix("/tag/")
            .is_some_and(|tag| !tag.is_empty() && tag.len() <= 64 && tag.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))),
    }
}

#[tauri::command]
pub fn open_release_page(url: String) -> Result<(), String> {
    if !is_allowed_release_url(&url) {
        return Err("Only this project's releases page can be opened".to_string());
    }
    open_in_browser(&url)
}

// ---------------------------------------------------------------------------------------------
// Fixed links and log export
// ---------------------------------------------------------------------------------------------

/// The pages the UI may open, by name. The page only ever sends the name, never a URL.
pub fn link_for(key: &str) -> Option<&'static str> {
    match key {
        "repo" => Some(REPO_URL),
        "discord_developer_portal" => Some("https://discord.com/developers/applications"),
        _ => None,
    }
}

#[tauri::command]
pub fn open_link(key: String) -> Result<(), String> {
    let url = link_for(&key).ok_or_else(|| format!("Unknown link: {key}"))?;
    open_in_browser(url)
}

/// A safe default name for the exported log: letters, digits, dots, dashes and underscores, ending in .txt.
pub fn export_file_name(wanted: &str) -> String {
    let cleaned: String = wanted
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '-' })
        .collect();
    let cleaned = cleaned.trim_matches(|c| c == '.' || c == '-').to_string();
    let base = if cleaned.is_empty() { "quest-completer-log".to_string() } else { cleaned };
    let base: String = base.chars().take(80).collect();
    if base.to_lowercase().ends_with(".txt") { base } else { format!("{base}.txt") }
}

/// Write the exported log to the path the user chose.
fn write_log_file(path: &Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| format!("Couldn't save the log: {e}"))
}

/// Ask where to save the log and write it there. The save dialog is shown by the backend, so the
/// page never gets to choose a path itself. Returns the saved path, or none if the user cancelled.
#[tauri::command(rename_all = "snake_case")]
pub async fn export_log(app: AppHandle, contents: String, file_name: String) -> Result<Option<String>, String> {
    if contents.len() > 5_000_000 {
        return Err("The log is too large to export".to_string());
    }
    let file_name = export_file_name(&file_name);
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        let Some(chosen) = app
            .dialog()
            .file()
            .set_file_name(&file_name)
            .add_filter("Text file", &["txt"])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = chosen.into_path().map_err(|e| e.to_string())?;
        write_log_file(&path, &contents)?;
        Ok(Some(path.to_string_lossy().to_string()))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------------------------
// Start with Windows
// ---------------------------------------------------------------------------------------------

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "Discord Quest Completer";

/// The command Windows runs at sign-in: the exe in quotes, plus `--minimized` to start in the tray.
pub fn autostart_command(exe: &Path, minimized: bool) -> String {
    format!("\"{}\"{}", exe.display(), if minimized { " --minimized" } else { "" })
}

fn reg(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("reg")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run reg: {}", e))
}

fn autostart_is_set(value: &str) -> bool {
    reg(&["query", RUN_KEY, "/v", value]).map(|out| out.status.success()).unwrap_or(false)
}

fn autostart_write(value: &str, command: Option<&str>) -> Result<(), String> {
    let out = match command {
        Some(command) => reg(&["add", RUN_KEY, "/v", value, "/t", "REG_SZ", "/d", command, "/f"])?,
        None if !autostart_is_set(value) => return Ok(()), // already off
        None => reg(&["delete", RUN_KEY, "/v", value, "/f"])?,
    };
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("Windows refused the change: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

#[tauri::command]
pub async fn get_autostart() -> bool {
    tauri::async_runtime::spawn_blocking(|| autostart_is_set(RUN_VALUE)).await.unwrap_or(false)
}

#[tauri::command]
pub async fn set_autostart(enabled: bool, minimized: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            autostart_write(RUN_VALUE, Some(&autostart_command(&exe, minimized)))
        } else {
            autostart_write(RUN_VALUE, None)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_every_discord_client_in_the_task_list() {
        let list = "\"System\",\"4\",\"Services\",\"0\",\"8 K\"\r\n\
                    \"Discord.exe\",\"1234\",\"Console\",\"1\",\"250,000 K\"\r\n\
                    \"Discord.exe\",\"1240\",\"Console\",\"1\",\"90,000 K\"\r\n\
                    \"DiscordCanary.exe\",\"77\",\"Console\",\"1\",\"1 K\"\r\n\
                    \"notdiscord.exe\",\"5\",\"Console\",\"1\",\"1 K\"\r\n";
        let status = parse_discord_status(list);
        assert!(status.running);
        assert_eq!(status.clients, vec!["Discord".to_string(), "Discord Canary".to_string()]);

        assert_eq!(parse_discord_status("\"DISCORDPTB.EXE\",\"9\",\"Console\",\"1\",\"1 K\"").clients, vec!["Discord PTB".to_string()]);
        for none in ["", "\"System\",\"4\"", "\"Discord Quest Completer.exe\",\"1\",\"Console\",\"1\",\"1 K\"", "garbage"] {
            assert_eq!(parse_discord_status(none), DiscordStatus::default(), "{none:?}");
        }
    }

    #[test]
    fn the_real_task_list_is_readable() {
        // Whatever is running, asking must not fail; the System process proves the list was parsed.
        let out = Command::new("tasklist").args(["/FO", "CSV", "/NH"]).creation_flags(CREATE_NO_WINDOW).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.to_lowercase().contains("\"system\""));
        let _ = parse_discord_status(&text);
    }

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("26.10.2", "26.10.1"));
        assert!(is_newer("v26.11.0", "26.10.9"));
        assert!(is_newer("27.1.0", "26.12.30"));
        assert!(is_newer("26.10.10", "26.10.9"), "10 is higher than 9, not a text comparison");
        assert!(is_newer("26.10.1.1", "26.10.1"));
        assert!(!is_newer("26.10.1", "26.10.1"));
        assert!(!is_newer("26.10.0", "26.10.1"));
        assert!(!is_newer("26.10", "26.10.0"), "missing parts count as zero");
        for bad in ["", "latest", "v", "1..2", "1.x.3", "26.10.1-beta"] {
            assert!(!is_newer(bad, "26.10.1"), "{bad:?} must never count as newer");
            assert!(!is_newer("99.0.0", bad), "{bad:?} as the current version must not claim an update");
        }
    }

    #[test]
    fn reads_the_latest_release() {
        let json = serde_json::json!({
            "tag_name": "v26.11.0",
            "html_url": "https://github.com/cmxau/discord-quest-completer/releases/tag/v26.11.0",
            "draft": false
        });
        assert_eq!(
            parse_release(&json),
            Some(("26.11.0".to_string(), "https://github.com/cmxau/discord-quest-completer/releases/tag/v26.11.0".to_string()))
        );
        // no page given: build the tag link ourselves
        let (_, url) = parse_release(&serde_json::json!({ "tag_name": "v1.2.3" })).unwrap();
        assert_eq!(url, "https://github.com/cmxau/discord-quest-completer/releases/tag/v1.2.3");
        // anything without a readable version is ignored
        assert_eq!(parse_release(&serde_json::json!({ "tag_name": "nightly" })), None);
        assert_eq!(parse_release(&serde_json::json!({ "message": "Not Found" })), None);
        assert_eq!(parse_release(&serde_json::Value::Null), None);
    }

    #[test]
    fn the_running_version_is_a_readable_version() {
        assert!(parse_version(env!("CARGO_PKG_VERSION")).is_some());
    }

    #[test]
    fn only_this_projects_releases_can_be_opened() {
        let ok = [
            "https://github.com/cmxau/discord-quest-completer/releases",
            "https://github.com/cmxau/discord-quest-completer/releases/latest",
            "https://github.com/cmxau/discord-quest-completer/releases/tag/v26.10.2",
        ];
        for url in ok {
            assert!(is_allowed_release_url(url), "should allow {url}");
        }
        let bad = [
            "",
            "https://example.com",
            "http://github.com/cmxau/discord-quest-completer/releases",
            "https://github.com/cmxau/discord-quest-completer",
            "https://github.com/cmxau/discord-quest-completer/releases/",
            "https://github.com/cmxau/discord-quest-completer/releasesx",
            "https://github.com/cmxau/discord-quest-completer/releases/download/v1/app.exe",
            "https://github.com/cmxau/discord-quest-completer/releases/tag/",
            "https://github.com/cmxau/discord-quest-completer/releases/tag/v1/../../x",
            "https://github.com/cmxau/discord-quest-completer/releases/tag/v1?x=1",
            "https://github.com/cmxau/discord-quest-completer/releases/tag/v1 2",
            "https://github.com/cmxau/discord-quest-completer/releases#x",
            "https://github.com.evil.com/cmxau/discord-quest-completer/releases",
            "https://github.com/other/repo/releases",
            "file:///C:/Windows/System32/calc.exe",
        ];
        for url in bad {
            assert!(!is_allowed_release_url(url), "should refuse {url:?}");
        }
    }

    #[test]
    fn saved_behavior_falls_back_to_the_defaults() {
        let defaults = Behavior { close_to_tray: false, stop_on_exit: true, keep_steam_entries: false };
        assert_eq!(parse_behavior(""), defaults);
        assert_eq!(parse_behavior("not json"), defaults);
        assert_eq!(parse_behavior("{}"), defaults);
        assert_eq!(parse_behavior(r#"{"keep_steam_entries": "yes"}"#), defaults, "a wrong type is ignored, not half applied");

        let saved = Behavior { close_to_tray: true, stop_on_exit: false, keep_steam_entries: true };
        assert_eq!(parse_behavior(&serde_json::to_string(&saved).unwrap()), saved);
        // a file from an older version with fewer fields keeps the defaults for the rest
        assert_eq!(parse_behavior(r#"{"close_to_tray": true}"#), Behavior { close_to_tray: true, ..defaults });
    }

    #[test]
    fn only_known_links_can_be_opened() {
        assert_eq!(link_for("repo"), Some("https://github.com/cmxau/discord-quest-completer"));
        assert_eq!(link_for("discord_developer_portal"), Some("https://discord.com/developers/applications"));
        for bad in ["", "REPO", "https://example.com", "repo ", "../repo", "file:///C:/Windows/System32/calc.exe"] {
            assert_eq!(link_for(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_exported_log_is_written_exactly() {
        let dir = std::env::temp_dir().join(format!("dqc-export-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(export_file_name("quest-completer-log-2026-10-10.txt"));

        // text with accents, symbols, emoji and both kinds of line ending survives byte for byte
        let log = "Discord Quest Completer log\r\n[12:00:01] [INFO] Playing AION 2 \u{2122} \u{1F3AE} caf\u{e9}\n[12:00:02] [ERROR] x\n";
        write_log_file(&path, log).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), log.as_bytes());

        // saving over an older export replaces it
        write_log_file(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");

        // a folder that doesn't exist is reported, not hidden
        let missing = dir.join("nope").join("log.txt");
        assert!(write_log_file(&missing, "x").unwrap_err().contains("Couldn't save the log"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn exported_file_names_are_safe() {
        assert_eq!(export_file_name("quest-completer-log.txt"), "quest-completer-log.txt");
        assert_eq!(export_file_name("my log"), "my-log.txt");
        assert_eq!(export_file_name("..\\..\\evil"), "evil.txt");
        assert_eq!(export_file_name("a/b:c*d?.TXT"), "a-b-c-d-.TXT");
        assert_eq!(export_file_name(""), "quest-completer-log.txt");
        assert_eq!(export_file_name("..."), "quest-completer-log.txt");
        assert!(export_file_name(&"x".repeat(500)).len() <= 84);
        for name in ["con", "a\u{0}b", "\u{202e}x", "x.exe"] {
            let out = export_file_name(name);
            assert!(out.ends_with(".txt") && out.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')), "{out:?}");
        }
    }

    #[test]
    fn the_autostart_command_quotes_the_path() {
        let exe = Path::new(r"C:\Program Files\Quest Completer\app.exe");
        assert_eq!(autostart_command(exe, false), r#""C:\Program Files\Quest Completer\app.exe""#);
        assert_eq!(autostart_command(exe, true), r#""C:\Program Files\Quest Completer\app.exe" --minimized"#);
    }

    #[test]
    fn autostart_round_trips_through_the_registry() {
        // A throwaway value name, removed again, so the real setting is never touched.
        let value = format!("DQC test {}", std::process::id());
        let exe = Path::new(r"C:\Program Files\Quest Completer\app.exe");
        assert!(!autostart_is_set(&value));

        autostart_write(&value, Some(&autostart_command(exe, true))).unwrap();
        assert!(autostart_is_set(&value));
        // what Windows will run is exactly the quoted command (spaces and quotes survive)
        let out = reg(&["query", RUN_KEY, "/v", &value]).unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).contains(r#""C:\Program Files\Quest Completer\app.exe" --minimized"#));

        autostart_write(&value, None).unwrap();
        assert!(!autostart_is_set(&value));
        autostart_write(&value, None).unwrap(); // turning off twice is fine
    }
}
