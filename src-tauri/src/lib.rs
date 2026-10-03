// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use once_cell::sync::OnceCell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{path::BaseDirectory, AppHandle, Emitter, Listener, Manager};

mod rpc;
mod runner;
mod steam;

// Global static instance of the Discord client
static DISCORD_CLIENT: OnceCell<Mutex<Option<rpc::Client>>> = OnceCell::new();

fn get_discord_client() -> &'static Mutex<Option<rpc::Client>> {
    DISCORD_CLIENT.get_or_init(|| Mutex::new(None))
}

fn runner_resource_name() -> &'static str {
    "data/quest-runner.exe"
}

// Discord application IDs are 64-bit snowflakes. They are passed as strings because a JS number
// can't represent them exactly, and are validated since they become a folder name.
fn game_folder_path(exe_dir: &Path, path: &str, app_id: &str) -> Result<PathBuf, String> {
    if app_id.is_empty() || !app_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("Invalid app id: {}", app_id));
    }

    let normalized_path = Path::new(path).to_string_lossy().to_string();

    Ok(exe_dir.join("games").join(app_id).join(normalized_path))
}

fn resolve_runner_template(handle: &AppHandle) -> Result<PathBuf, String> {
    handle
        .path()
        .resolve(runner_resource_name(), BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve runner template: {}", e))
}

#[tauri::command(rename_all = "snake_case")]
async fn create_fake_game(
    handle: tauri::AppHandle,
    path: &str,
    executable_name: &str,
    app_id: String,
) -> Result<String, String> {
    // Must create in the same directory as the executable to avoid permission issues
    // Get the executable directory to look for config file
    let exe_path: std::path::PathBuf = env::current_exe().unwrap_or_default();
    let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new(""));

    let game_folder_path = game_folder_path(exe_dir, path, &app_id)?;

    println!("Game folder path: {:?}", game_folder_path);
    println!(
        "Game full path: {:?}",
        game_folder_path.join(executable_name)
    );

    match fs::create_dir_all(&game_folder_path) {
        Ok(_) => {
            println!("Successfully created directory: {:?}", game_folder_path);
        }
        Err(e) => return Err(format!("Failed to create game folder: {}", e)),
    };

    let resource_path = resolve_runner_template(&handle)?;
    println!("Creating dummy game executable from: {:?}", resource_path);

    let target_executable_path = game_folder_path.join(executable_name);
    match fs::copy(&resource_path, &target_executable_path) {
        Ok(_) => Ok(format!(
            "Dummy executable copied to: {:?}",
            target_executable_path
        )),
        Err(e) => Err(format!("Failed to copy dummy executable: {}", e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn run_background_process(
    name: &str,
    path: &str,
    executable_name: &str,
    app_id: String,
) -> Result<String, String> {
    let exe_path = env::current_exe().unwrap_or_default();
    let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new(""));

    let game_folder_path = game_folder_path(exe_dir, path, &app_id)?;
    let executable_path = game_folder_path.join(executable_name);

    let mut cmd = std::process::Command::new(&executable_path);
    cmd.args(["--title", name]).current_dir(game_folder_path);

    match cmd.spawn() {
        Ok(_) => Ok("Process started successfully".to_string()),
        Err(e) => Err(format!("Failed to start process: {}", e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn stop_process(exec_name: String) -> Result<(), String> {
    let output = std::process::Command::new("taskkill")
        .arg("/F")
        .arg("/IM")
        .arg(&exec_name)
        .output()
        .map_err(|e| format!("Failed to execute taskkill: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to stop process: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Lock the global client slot, recovering if a previous holder panicked.
fn lock_discord_client() -> std::sync::MutexGuard<'static, Option<rpc::Client>> {
    get_discord_client()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Usage: Calling from JS:
/// ```javascript
/// await invoke('connect_to_discord_rpc_3', { activity_json });
/// emit('event_disconnect');
/// ```
/// Emits `client_connecting`, then `client_connected` or `client_error` (e.g. Discord not running).
#[tauri::command(rename_all = "snake_case")]
fn connect_to_discord_rpc_3(handle: AppHandle, activity_json: String) -> Result<(), String> {
    let app = handle.clone();

    let event_connecting = "client_connecting";
    let event_connected = "client_connected";
    let event_error = "client_error";
    let event_disconnect = "event_disconnect";

    let activity = runner::parse_activity_json(&activity_json)?;

    let connecting_payload = serde_json::json!({
        "app_id": activity.app_id,
    });

    // Drop any previous client so only one activity is active at a time.
    drop(lock_discord_client().take());

    let task = tauri::async_runtime::spawn(async move {
        handle
            .emit(event_connecting, connecting_payload)
            .unwrap_or_else(|e| eprintln!("Failed to emit event: {}", e));

        let client = match runner::set_activity(activity_json).await {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Failed to set activity: {}", e);
                handle
                    .emit(event_error, serde_json::json!({ "message": e }))
                    .unwrap_or_else(|e| eprintln!("Failed to emit event: {}", e));
                return;
            }
        };

        let connected_payload = serde_json::json!({
            "app_id": activity.app_id,
        });

        *lock_discord_client() = Some(client);

        handle
            .emit(event_connected, connected_payload)
            .unwrap_or_else(|e| {
                eprintln!("Failed to emit event: {}", e);
            });

        handle.listen(event_disconnect, move |_| {
            println!("Disconnecting from Discord RPC inner");
            tauri::async_runtime::spawn(async move {
                let client_option = lock_discord_client().take();
                if let Some(client) = client_option {
                    client.discord.disconnect().await;
                    println!("Disconnected from Discord RPC inner");
                }
            });
        });
    });

    app.listen(event_disconnect, move |_| {
        println!("Disconnecting from Discord RPC...");
        task.abort();
    });

    Ok(())
}

// Return errors instead of unwrapping so the frontend can fall back to the next game list source.
async fn fetch_text(url: &str) -> Result<tauri::ipc::Response, String> {
    let result = async {
        let res = tauri_plugin_http::reqwest::get(url)
            .await
            .map_err(|e| format!("Request to {} failed: {}", url, e))?;
        let res = res
            .error_for_status()
            .map_err(|e| format!("Request to {} failed: {}", url, e))?;
        res.text()
            .await
            .map_err(|e| format!("Failed to read response from {}: {}", url, e))
    }
    .await;

    match &result {
        Ok(body) => println!("Fetched game list from {} ({} bytes)", url, body.len()),
        Err(e) => eprintln!("Game list fetch failed: {}", e),
    }
    result.map(tauri::ipc::Response::new)
}

#[tauri::command(rename_all = "snake_case")]
async fn fetch_gamelist_gh_mirror() -> Result<tauri::ipc::Response, String> {
    fetch_text("https://markterence.github.io/discord-quest-completer/detectable.json").await
}

#[tauri::command(rename_all = "snake_case")]
async fn fetch_gamelist_from_discord() -> Result<tauri::ipc::Response, String> {
    fetch_text("https://discord.com/api/applications/detectable").await
}

fn steam_registry_path(handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data folder: {}", e))?;
    Ok(dir.join("steam_fakes.json"))
}

/// Install folder and launch exe for a Steam app (from the community steamcmd.net mirror), plus
/// whether Steam is installed here. Network problems are not an error: the fields are just empty.
#[tauri::command(rename_all = "snake_case")]
async fn steam_game_info(steam_id: String) -> Result<steam::SteamGameInfo, String> {
    if steam_id.is_empty() || !steam_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("Invalid Steam app id: {}", steam_id));
    }
    let mut info = steam::SteamGameInfo {
        steam_found: steam::find_steamapps_dir().is_some(),
        ..Default::default()
    };

    let url = format!("https://api.steamcmd.net/v1/info/{}", steam_id);
    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let body = match client.get(&url).send().await {
        Ok(res) => res.text().await.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    match body.and_then(|text| serde_json::from_str::<serde_json::Value>(&text).map_err(|e| e.to_string())) {
        Ok(json) => {
            let (installdir, exe) = steam::parse_steamcmd_info(&json, &steam_id);
            info.installdir = installdir;
            info.exe = exe;
        }
        Err(e) => eprintln!("Steam info lookup failed for {}: {}", steam_id, e),
    }
    Ok(info)
}

/// Create a fake Steam install for `steam_id` (manifest + dummy exe) and start the exe.
/// Returns the exe file name, which is what `stop_steam_game` needs.
#[tauri::command(rename_all = "snake_case")]
async fn launch_steam_game(
    handle: AppHandle,
    steam_id: String,
    name: String,
    install_dir: String,
    exe_name: String,
) -> Result<String, String> {
    let steamapps = steam::find_steamapps_dir().ok_or_else(|| "Steam installation not found".to_string())?;
    let registry = steam_registry_path(&handle)?;
    let runner = resolve_runner_template(&handle)?;

    let prepared = steam::prepare_fake(&steamapps, &registry, &runner, &steam_id, &name, &install_dir, &exe_name)?;

    let file_name = prepared
        .exe_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or(exe_name);

    match std::process::Command::new(&prepared.exe_path)
        .args(["--title", &name])
        .current_dir(&prepared.game_dir)
        .spawn()
    {
        Ok(_) => {}
        Err(e) => {
            let _ = steam::cleanup_fake(&steamapps, &registry, &steam_id);
            return Err(format!("Failed to start process: {}", e));
        }
    }

    Ok(file_name)
}

/// Stop the fake game's process and remove the manifest, folder and exe we created for it.
#[tauri::command(rename_all = "snake_case")]
async fn stop_steam_game(handle: AppHandle, steam_id: String, exe_filename: String) -> Result<(), String> {
    let steamapps = steam::find_steamapps_dir().ok_or_else(|| "Steam installation not found".to_string())?;
    let registry = steam_registry_path(&handle)?;

    tauri::async_runtime::spawn_blocking(move || {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", &exe_filename])
            .output();

        // The process needs a moment to release the exe before it can be deleted.
        let mut last_error = String::new();
        for _ in 0..10 {
            match steam::cleanup_fake(&steamapps, &registry, &steam_id) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = e;
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
            }
        }
        Err(last_error)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            // Remove fake Steam installs left behind if the app was closed while a game was running.
            if let (Some(steamapps), Ok(registry)) = (steam::find_steamapps_dir(), steam_registry_path(app.handle())) {
                let (cleaned, kept) = steam::cleanup_all(&steamapps, &registry);
                if cleaned + kept > 0 {
                    println!("Steam fake cleanup: {} removed, {} still in use", cleaned, kept);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_fake_game,
            stop_process,
            connect_to_discord_rpc_3,
            run_background_process,
            fetch_gamelist_gh_mirror,
            fetch_gamelist_from_discord,
            steam_game_info,
            launch_steam_game,
            stop_steam_game
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
