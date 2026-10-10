//! Launch dummy "games" from inside the user's Steam library.
//!
//! Discord's newer quest detection pairs a running exe with a Steam install, so the dummy exe must
//! live in `steamapps/common/<installdir>/` next to an `appmanifest_<id>.acf`. Because that is the
//! user's real Steam library, everything created here is recorded in a registry file and removed
//! again on stop (or on the next start if the app was closed while the game was running).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// One fake install we created, with exactly what to remove again.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SteamFake {
    pub steam_id: String,
    pub acf_path: PathBuf,
    /// False if the manifest already existed and was reused (it is then left alone on cleanup).
    pub acf_created: bool,
    pub exe_path: PathBuf,
    /// Directories we created (outermost first). Only removed if empty.
    pub created_dirs: Vec<PathBuf>,
}

#[derive(Serialize, Debug, Default)]
pub struct SteamGameInfo {
    pub steam_found: bool,
    pub installdir: Option<String>,
    pub exe: Option<String>,
    /// True when Steam's data could not be fetched, so the fields above are empty for that reason.
    pub lookup_failed: bool,
}

/// What `launch_steam_game` reports back to the UI.
#[derive(Serialize, Debug)]
pub struct LaunchedSteamGame {
    /// The exe's file name, which `stop_steam_game` needs.
    pub file_name: String,
    /// Where the dummy exe was created, shown so it is clear what was added.
    pub exe_path: String,
    /// True when the manifest was filled from Steam's build and depot data, false when it is minimal.
    pub manifest_from_steam: bool,
    /// Where the Steam manifest was written (in `steamapps`, one level above the game folder).
    pub manifest_path: String,
}

#[derive(Debug)]
pub struct PreparedGame {
    pub exe_path: PathBuf,
    pub game_dir: PathBuf,
    pub acf_path: PathBuf,
}

// ---------------------------------------------------------------------------------------------
// Locating Steam
// ---------------------------------------------------------------------------------------------

/// Locate the local Steam `steamapps` folder, if Steam is installed.
pub fn find_steamapps_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // SteamPath is stored in the registry, e.g. "c:/program files (x86)/steam"
    if let Ok(output) = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = text.lines().find(|l| l.contains("SteamPath")) {
            if let Some(idx) = line.find("REG_SZ") {
                candidates.push(PathBuf::from(line[idx + "REG_SZ".len()..].trim()));
            }
        }
    }
    if let Ok(pf) = std::env::var("ProgramFiles(x86)") {
        candidates.push(PathBuf::from(pf).join("Steam"));
    }

    candidates
        .into_iter()
        .map(|p| p.join("steamapps"))
        .find(|p| p.is_dir())
}

// ---------------------------------------------------------------------------------------------
// Validation (everything below ends up as a path inside the user's Steam library)
// ---------------------------------------------------------------------------------------------

fn validate_steam_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 10 || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("Invalid Steam app id: {}", id));
    }
    Ok(())
}

/// A single path component: no separators, no reserved characters, not `.`/`..`.
fn validate_component(name: &str) -> Result<String, String> {
    let name = name.trim();
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.chars().count() > 100 // characters, not bytes: a 100-character CJK title is 300 bytes
        || name.ends_with('.')
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'));
    if bad {
        return Err(format!("Invalid folder or file name: {:?}", name));
    }
    Ok(name.to_string())
}

/// A relative exe path such as `fc27.exe` or `Shipping/Product/REDSteam.exe`.
fn validate_relative_exe(exe: &str) -> Result<PathBuf, String> {
    let normalized = exe.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() || parts.len() > 5 {
        return Err(format!("Invalid executable path: {:?}", exe));
    }
    let mut path = PathBuf::new();
    for part in &parts {
        path.push(validate_component(part)?);
    }
    if !parts.last().unwrap().to_ascii_lowercase().ends_with(".exe") {
        return Err("The executable name must end in .exe".to_string());
    }
    Ok(path)
}

fn is_within(path: &Path, base: &Path) -> bool {
    path.starts_with(base)
        && !path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
}

fn acf_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

// ---------------------------------------------------------------------------------------------
// Registry of what we created
// ---------------------------------------------------------------------------------------------

fn load_registry(path: &Path) -> Result<Vec<SteamFake>, String> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("Steam fake registry {:?} is corrupted: {}", path, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("Failed to read Steam fake registry: {}", e)),
    }
}

fn save_registry(path: &Path, entries: &[SteamFake]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create data folder: {}", e))?;
    }
    let text = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| format!("Failed to write Steam fake registry: {}", e))
}

/// What the "Steam entries" panel shows for one fake install.
#[derive(Serialize, Debug, PartialEq)]
pub struct FakeSummary {
    pub steam_id: String,
    /// The game's folder under `steamapps/common` (empty if it can't be told).
    pub folder_name: String,
    pub exe_name: String,
    pub exe_path: String,
    pub acf_path: String,
}

/// Every fake install currently recorded in the registry.
pub fn list_fakes(registry_path: &Path) -> Result<Vec<FakeSummary>, String> {
    Ok(load_registry(registry_path)?
        .into_iter()
        .map(|fake| {
            let mut parts = fake.exe_path.components().map(|c| c.as_os_str().to_string_lossy().to_string());
            let folder_name = parts
                .by_ref()
                .find(|part| part.eq_ignore_ascii_case("common"))
                .and_then(|_| parts.next())
                .unwrap_or_default();
            FakeSummary {
                steam_id: fake.steam_id,
                folder_name,
                exe_name: fake.exe_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                exe_path: fake.exe_path.to_string_lossy().to_string(),
                acf_path: fake.acf_path.to_string_lossy().to_string(),
            }
        })
        .collect())
}

// ---------------------------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------------------------

/// One depot of an app as Steam lists it for the public branch.
#[derive(Debug, Clone, PartialEq)]
pub struct SteamDepot {
    pub id: String,
    pub manifest: String,
    pub size: String,
}

/// What a real install of the app would record: the current public build and its depots.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SteamBuild {
    pub build_id: String,
    pub depots: Vec<SteamDepot>,
    /// Depots shared with another app (for example Steamworks redistributables), as (depot, app).
    pub shared_depots: Vec<(String, String)>,
}

impl SteamBuild {
    fn total_size(&self) -> u64 {
        self.depots.iter().filter_map(|d| d.size.parse::<u64>().ok()).sum()
    }
}

/// Pull the public build id and depot list out of a steamcmd.net `info` response.
pub fn parse_steam_build(json: &serde_json::Value, steam_id: &str) -> Option<SteamBuild> {
    let depots = &json["data"][steam_id]["depots"];
    let build_id = depots["branches"]["public"]["buildid"].as_str().filter(|b| !b.is_empty())?.to_string();

    let mut build = SteamBuild { build_id, ..Default::default() };
    for (key, value) in depots.as_object()? {
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_digit()) {
            continue; // "branches", "baselanguages", ...
        }
        if let Some(app) = value["depotfromapp"].as_str() {
            build.shared_depots.push((key.clone(), app.to_string()));
        } else if let Some(public) = value["manifests"]["public"].as_object() {
            if let Some(gid) = public.get("gid").and_then(|g| g.as_str()) {
                let size = public.get("size").and_then(|s| s.as_str()).unwrap_or("0");
                build.depots.push(SteamDepot { id: key.clone(), manifest: gid.to_string(), size: size.to_string() });
            }
        }
    }
    let by_id = |id: &String| id.parse::<u64>().unwrap_or(u64::MAX);
    build.depots.sort_by_key(|d| by_id(&d.id));
    build.shared_depots.sort_by_key(|(d, _)| by_id(d));
    if build.depots.is_empty() {
        return None; // nothing to describe, so keep the minimal manifest
    }
    Some(build)
}

/// The text of `appmanifest_<id>.acf`. With Steam's build data it looks like a finished install
/// (real build id, depots and size); without it, a minimal manifest that still names the install.
fn manifest_text(
    steam_id: &str,
    name: &str,
    install_dir: &str,
    steam_root: Option<&Path>,
    build: Option<&SteamBuild>,
) -> String {
    let last_updated = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let kv = |key: &str, value: &str| format!("\t\"{}\"\t\t\"{}\"\n", key, acf_escape(value));
    let mut out = String::from("\"AppState\"\n{\n");
    out += &kv("appid", steam_id);
    out += &kv("Universe", "1");
    if let Some(root) = steam_root {
        let launcher = root.join("steam.exe").to_string_lossy().replace('/', "\\");
        out += &kv("LauncherPath", &launcher);
    }
    out += &kv("name", name);
    // StateFlags 4 = fully installed.
    out += &kv("StateFlags", "4");
    out += &kv("installdir", install_dir);
    out += &kv("LastUpdated", &last_updated.to_string());
    out += &kv("LastPlayed", "0");
    out += &kv("SizeOnDisk", &build.map(|b| b.total_size()).unwrap_or(0).to_string());
    out += &kv("StagingSize", "0");
    out += &kv("buildid", build.map(|b| b.build_id.as_str()).unwrap_or("0"));
    out += &kv("LastOwner", "0");
    out += &kv("DownloadType", "0");
    out += &kv("UpdateResult", "0");
    out += &kv("BytesToDownload", "0");
    out += &kv("BytesDownloaded", "0");
    out += &kv("BytesToStage", "0");
    out += &kv("BytesStaged", "0");
    if let Some(b) = build {
        out += &kv("TargetBuildID", &b.build_id);
    }
    // 1 = "only update when I launch it", so Steam never starts downloading in the background.
    out += &kv("AutoUpdateBehavior", "1");
    out += &kv("AllowOtherDownloadsWhileRunning", "0");
    out += &kv("ScheduledAutoUpdate", "0");

    out += "\t\"InstalledDepots\"\n\t{\n";
    for depot in build.map(|b| b.depots.as_slice()).unwrap_or(&[]) {
        out += &format!("\t\t\"{}\"\n\t\t{{\n\t\t\t\"manifest\"\t\t\"{}\"\n\t\t\t\"size\"\t\t\"{}\"\n\t\t}}\n", depot.id, depot.manifest, depot.size);
    }
    out += "\t}\n";
    if let Some(b) = build.filter(|b| !b.shared_depots.is_empty()) {
        out += "\t\"SharedDepots\"\n\t{\n";
        for (depot, app) in &b.shared_depots {
            out += &format!("\t\t\"{}\"\t\t\"{}\"\n", depot, app);
        }
        out += "\t}\n";
    }
    out += "\t\"UserConfig\"\n\t{\n\t\t\"language\"\t\t\"english\"\n\t}\n";
    out += "\t\"MountedConfig\"\n\t{\n\t\t\"language\"\t\t\"english\"\n\t}\n";
    out += "}\n";
    out
}

/// Is this manifest an *empty* fake install (no game files, nothing downloaded) rather than a real one?
///
/// Older versions of this app wrote such manifests, and Steam later rewrites them with its own fields
/// (real name, `StateFlags` 6, ...). A real install always has a build id, a size and installed depots,
/// and a pending download has a target build, so none of those may be present.
fn is_empty_install_manifest(text: &str, steam_id: &str) -> bool {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('"').collect();
        // `"key"<tabs>"value"` splits into ["", key, tabs, value, ""]
        if parts.len() >= 5 {
            pairs.push((parts[1].to_string(), parts[3].to_string()));
        }
    }
    let get = |key: &str| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());

    get("appid") == Some(steam_id)
        && get("buildid") == Some("0")
        && get("SizeOnDisk") == Some("0")
        && !text.contains("\"manifest\"") // installed depots list their manifests
        && !text.contains("TargetBuildID") // a queued download
        && !text.contains("BytesToDownload")
}

fn dir_is_empty(path: &Path) -> bool {
    fs::read_dir(path).map(|mut d| d.next().is_none()).unwrap_or(false)
}

// ---------------------------------------------------------------------------------------------
// Prepare / cleanup
// ---------------------------------------------------------------------------------------------

/// Create the fake install (manifest + dummy exe) and record it. Refuses to touch anything that
/// looks like a real install. (The app itself always goes through `prepare_fake_with_build`.)
#[cfg(test)]
pub fn prepare_fake(
    steamapps: &Path,
    registry_path: &Path,
    runner: &Path,
    steam_id: &str,
    name: &str,
    install_dir: &str,
    exe: &str,
) -> Result<PreparedGame, String> {
    prepare_fake_with_build(steamapps, registry_path, runner, steam_id, name, install_dir, exe, None)
}

/// Like `prepare_fake`, and with Steam's build data the manifest records a real build id, depots
/// and size instead of an empty install.
pub fn prepare_fake_with_build(
    steamapps: &Path,
    registry_path: &Path,
    runner: &Path,
    steam_id: &str,
    name: &str,
    install_dir: &str,
    exe: &str,
    build: Option<&SteamBuild>,
) -> Result<PreparedGame, String> {
    validate_steam_id(steam_id)?;
    let install_dir = validate_component(install_dir)?;
    let rel_exe = validate_relative_exe(exe)?;
    let name = name.trim();
    if name.is_empty() {
        return Err("Game name is empty".to_string());
    }

    let common = steamapps.join("common");
    let game_dir = common.join(&install_dir);
    let exe_path = game_dir.join(&rel_exe);
    let acf_path = steamapps.join(format!("appmanifest_{}.acf", steam_id));
    if !is_within(&exe_path, &common) {
        return Err("Executable path escapes the Steam library".to_string());
    }

    let mut registry = load_registry(registry_path)?;
    // A kept entry for this game with a different folder or exe name: remove the old files first,
    // or they would be left behind untracked.
    if registry.iter().any(|e| e.steam_id == steam_id && e.exe_path != exe_path) {
        cleanup_fake(steamapps, registry_path, steam_id)?;
        registry = load_registry(registry_path)?;
    }
    let existing = registry.iter().position(|e| e.steam_id == steam_id);

    // Decide what is ours and what is not before writing anything.
    let mut acf_created = true;
    let mut adopted_dir = false;
    let mut adopted_acf = false;
    if existing.is_none() {
        if acf_path.exists() {
            let legacy = fs::read_to_string(&acf_path)
                .map(|t| is_empty_install_manifest(&t, steam_id))
                .unwrap_or(false);
            if legacy && (!game_dir.exists() || dir_is_empty(&game_dir)) {
                // Empty fake install left by an earlier run: take ownership so it gets cleaned up.
                adopted_dir = game_dir.exists();
                adopted_acf = true;
            } else {
                return Err(format!(
                    "Steam already has a manifest for app {} (is the game really installed?). Not touching it.",
                    steam_id
                ));
            }
        } else if game_dir.exists() && !dir_is_empty(&game_dir) {
            return Err(format!(
                "The folder {:?} already exists and is not empty. Not touching it.",
                game_dir
            ));
        }
    } else {
        acf_created = registry[existing.unwrap()].acf_created;
    }

    // Directories we will create, outermost first.
    let mut created_dirs: Vec<PathBuf> = Vec::new();
    let mut cursor = common.clone();
    let mut chain = vec![common.clone()];
    for comp in exe_path
        .parent()
        .unwrap_or(&game_dir)
        .strip_prefix(&common)
        .map_err(|_| "Executable path escapes the Steam library".to_string())?
        .components()
    {
        cursor = cursor.join(comp);
        chain.push(cursor.clone());
    }
    for dir in chain {
        if !dir.exists() || (adopted_dir && dir == game_dir) {
            created_dirs.push(dir);
        }
    }

    // Record first, so a crash half-way never leaves untracked files behind.
    let entry = SteamFake {
        steam_id: steam_id.to_string(),
        acf_path: acf_path.clone(),
        acf_created,
        exe_path: exe_path.clone(),
        created_dirs: match existing {
            Some(i) => registry[i].created_dirs.clone(),
            None => created_dirs.clone(),
        },
    };
    match existing {
        Some(i) => registry[i] = entry,
        None => registry.push(entry),
    }
    save_registry(registry_path, &registry)?;

    let result = (|| -> Result<(), String> {
        fs::create_dir_all(exe_path.parent().unwrap_or(&game_dir))
            .map_err(|e| format!("Failed to create game folder: {}", e))?;
        fs::copy(runner, &exe_path).map_err(|e| format!("Failed to copy dummy executable: {}", e))?;

        // A manifest we created in an earlier launch (or an old-format leftover we adopted) is
        // replaced, so every launch writes the current, most complete version.
        if (adopted_acf || (existing.is_some() && acf_created)) && acf_path.exists() {
            fs::remove_file(&acf_path).map_err(|e| format!("Failed to replace old manifest: {}", e))?;
        }
        if !acf_path.exists() {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true) // never overwrite a manifest that appeared in the meantime
                .open(&acf_path)
                .map_err(|e| format!("Failed to create Steam manifest: {}", e))?;
            file.write_all(manifest_text(steam_id, name, &install_dir, steamapps.parent(), build).as_bytes())
                .map_err(|e| format!("Failed to write Steam manifest: {}", e))?;
        }
        Ok(())
    })();

    if let Err(e) = result {
        let _ = cleanup_fake(steamapps, registry_path, steam_id);
        return Err(e);
    }

    Ok(PreparedGame { exe_path, game_dir, acf_path })
}


/// What the "Open folder" button should show in Explorer.
#[derive(Debug, PartialEq)]
pub enum OpenTarget {
    /// Open the folder containing this file and highlight it.
    Select(PathBuf),
    /// Open this folder.
    Folder(PathBuf),
}

/// Decide what to open for a game. Never leaves the Steam library: a file outside
/// `steamapps/common` is ignored, and the install folder name is validated like any other.
pub fn resolve_open_target(
    steamapps: &Path,
    install_dir: &str,
    exe_path: Option<&str>,
) -> Result<OpenTarget, String> {
    let common = steamapps.join("common");

    // While a fake install is running, show the exe itself.
    if let Some(exe) = exe_path.map(str::trim).filter(|p| !p.is_empty()) {
        let exe = PathBuf::from(exe);
        if is_within(&exe, &common) && exe.is_file() {
            return Ok(OpenTarget::Select(exe));
        }
    }

    let game_dir = common.join(validate_component(install_dir)?);
    if game_dir.is_dir() {
        Ok(OpenTarget::Folder(game_dir))
    } else if common.is_dir() {
        // Nothing has been created yet: show the library folder the game would go into.
        Ok(OpenTarget::Folder(common))
    } else {
        Err("The Steam library folder was not found".to_string())
    }
}
/// Remove everything `prepare_fake` created for `steam_id`. Returns `Ok(false)` if nothing was
/// registered, and an error (keeping the registry entry) if the exe is still in use.
pub fn cleanup_fake(steamapps: &Path, registry_path: &Path, steam_id: &str) -> Result<bool, String> {
    validate_steam_id(steam_id)?;
    let mut registry = load_registry(registry_path)?;
    let Some(index) = registry.iter().position(|e| e.steam_id == steam_id) else {
        return Ok(false);
    };
    let entry = registry[index].clone();

    // The registry is a plain file: never delete anything outside the Steam library because of it.
    let common = steamapps.join("common");
    let safe = is_within(&entry.exe_path, &common)
        && entry.created_dirs.iter().all(|d| is_within(d, &common))
        && entry.acf_path == steamapps.join(format!("appmanifest_{}.acf", steam_id));
    if !safe {
        registry.remove(index);
        save_registry(registry_path, &registry)?;
        return Err("Ignored a registry entry that points outside the Steam library".to_string());
    }

    if entry.exe_path.exists() {
        fs::remove_file(&entry.exe_path).map_err(|e| {
            format!("Could not remove {:?} (is it still running?): {}", entry.exe_path, e)
        })?;
    }
    for dir in entry.created_dirs.iter().rev() {
        if dir.exists() && dir_is_empty(dir) {
            let _ = fs::remove_dir(dir);
        }
    }
    if entry.acf_created && entry.acf_path.exists() {
        fs::remove_file(&entry.acf_path)
            .map_err(|e| format!("Could not remove {:?}: {}", entry.acf_path, e))?;
    }

    registry.remove(index);
    save_registry(registry_path, &registry)?;
    Ok(true)
}

/// Clean up everything still registered (e.g. left behind if the app was closed while running).
/// Entries whose exe is still in use are kept. Returns (cleaned, kept).
pub fn cleanup_all(steamapps: &Path, registry_path: &Path) -> (usize, usize) {
    let ids: Vec<String> = match load_registry(registry_path) {
        Ok(entries) => entries.into_iter().map(|e| e.steam_id).collect(),
        Err(e) => {
            eprintln!("{}", e);
            return (0, 0);
        }
    };
    let (mut cleaned, mut kept) = (0, 0);
    for id in ids {
        match cleanup_fake(steamapps, registry_path, &id) {
            Ok(_) => cleaned += 1,
            Err(e) => {
                eprintln!("Steam fake {} kept: {}", id, e);
                kept += 1;
            }
        }
    }
    (cleaned, kept)
}

// ---------------------------------------------------------------------------------------------
// Steam metadata (install folder + launch exe), from the community steamcmd.net mirror
// ---------------------------------------------------------------------------------------------

/// Pull `installdir` and the first usable Windows launch exe out of a steamcmd.net `info` response.
pub fn parse_steamcmd_info(json: &serde_json::Value, steam_id: &str) -> (Option<String>, Option<String>) {
    let config = &json["data"][steam_id]["config"];
    let installdir = config["installdir"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());

    let mut launches: Vec<(u32, &serde_json::Value)> = config["launch"]
        .as_object()
        .map(|m| m.iter().filter_map(|(k, v)| k.parse().ok().map(|k| (k, v))).collect())
        .unwrap_or_default();
    launches.sort_by_key(|(k, _)| *k);

    let exe = launches.into_iter().find_map(|(_, entry)| {
        let exe = entry["executable"].as_str()?;
        if !exe.to_ascii_lowercase().ends_with(".exe") {
            return None; // e.g. a steam2ea:// launcher URL
        }
        if entry["config"]["betakey"].is_string() {
            return None; // beta / test branches
        }
        if let Some(oslist) = entry["config"]["oslist"].as_str() {
            if !oslist.contains("windows") {
                return None;
            }
        }
        // Steam writes launch paths with a leading slash ("/Aion2/Binaries/Win64/AION2.exe").
        Some(exe.replace('\\', "/").trim_start_matches('/').to_string())
    });

    (installdir, exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dqc-steam-test-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct Env {
        root: PathBuf,
        steamapps: PathBuf,
        registry: PathBuf,
        runner: PathBuf,
    }

    fn env(tag: &str) -> Env {
        let root = temp_dir(tag);
        let steamapps = root.join("steamapps");
        fs::create_dir_all(steamapps.join("common")).unwrap();
        let runner = root.join("runner.exe");
        fs::write(&runner, b"MZ-dummy").unwrap();
        Env { registry: root.join("data").join("steam_fakes.json"), root, steamapps, runner }
    }

    fn prepare(e: &Env, id: &str, dir: &str, exe: &str) -> Result<PreparedGame, String> {
        prepare_fake(&e.steamapps, &e.registry, &e.runner, id, "EA SPORTS FC™ 27", dir, exe)
    }

    #[test]
    fn creates_and_cleans_up_everything() {
        let e = env("roundtrip");
        let p = prepare(&e, "4080220", "EA SPORTS FC 27", "fc27.exe").unwrap();
        assert!(p.exe_path.is_file());
        let acf = e.steamapps.join("appmanifest_4080220.acf");
        let text = fs::read_to_string(&acf).unwrap();
        assert!(text.contains("\"appid\"\t\t\"4080220\""));
        assert!(text.contains("EA SPORTS FC™ 27"));
        assert!(text.contains("\"installdir\"\t\t\"EA SPORTS FC 27\""));

        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "4080220"), Ok(true));
        assert!(!acf.exists());
        assert!(!p.game_dir.exists());
        assert!(e.steamapps.join("common").exists(), "common/ existed before, must stay");
        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "4080220"), Ok(false));
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn launching_again_with_another_folder_removes_the_old_files() {
        let e = env("moved");
        let first = prepare(&e, "5", "Old Folder", "old.exe").unwrap();
        assert!(first.exe_path.exists());

        // a kept entry, launched again with different names
        let second = prepare(&e, "5", "New Folder", "new.exe").unwrap();
        assert!(second.exe_path.exists());
        assert!(!first.exe_path.exists(), "the old exe must not be left behind");
        assert!(!first.game_dir.exists(), "the old folder must not be left behind");
        assert_eq!(list_fakes(&e.registry).unwrap().len(), 1);

        // same names again is a plain re-launch and keeps working
        let third = prepare(&e, "5", "New Folder", "new.exe").unwrap();
        assert!(third.exe_path.exists() && third.acf_path.exists());

        cleanup_fake(&e.steamapps, &e.registry, "5").unwrap();
        assert!(!second.game_dir.exists() && !third.acf_path.exists());
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn lists_what_was_created() {
        let e = env("list");
        assert_eq!(list_fakes(&e.registry), Ok(vec![]), "no registry yet means nothing to list");

        prepare(&e, "4080220", "EA SPORTS FC 27", "fc27.exe").unwrap();
        let fakes = list_fakes(&e.registry).unwrap();
        assert_eq!(fakes.len(), 1);
        assert_eq!(fakes[0].steam_id, "4080220");
        assert_eq!(fakes[0].folder_name, "EA SPORTS FC 27");
        assert_eq!(fakes[0].exe_name, "fc27.exe");
        assert!(fakes[0].acf_path.ends_with("appmanifest_4080220.acf"));

        cleanup_fake(&e.steamapps, &e.registry, "4080220").unwrap();
        assert_eq!(list_fakes(&e.registry), Ok(vec![]));
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn refuses_real_installs() {
        let e = env("real");
        // Real manifest present
        fs::write(e.steamapps.join("appmanifest_1.acf"), "\"AppState\"\n{\n\t\"appid\"\t\t\"1\"\n\t\"buildid\"\t\t\"123\"\n}\n").unwrap();
        assert!(prepare(&e, "1", "Game", "game.exe").unwrap_err().contains("already has a manifest"));
        // Non-empty folder without manifest
        let dir = e.steamapps.join("common").join("Real Game");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("data.bin"), b"x").unwrap();
        assert!(prepare(&e, "2", "Real Game", "game.exe").unwrap_err().contains("not empty"));
        // Nothing was changed
        assert!(dir.join("data.bin").exists());
        assert!(e.steamapps.join("appmanifest_1.acf").exists());
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn leaves_a_reused_empty_folder_and_existing_dirs_alone() {
        let e = env("empty");
        let dir = e.steamapps.join("common").join("Empty Game");
        fs::create_dir_all(&dir).unwrap();
        prepare(&e, "3", "Empty Game", "x.exe").unwrap();
        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "3"), Ok(true));
        assert!(dir.is_dir(), "a folder that existed before must not be deleted");
        assert!(!dir.join("x.exe").exists());
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn adopts_and_removes_legacy_leftovers() {
        let e = env("legacy");
        let acf = e.steamapps.join("appmanifest_4080220.acf");
        fs::write(&acf, "\"AppState\"\n{\n\t\"appid\"\t\t\"4080220\"\n\t\"Universe\"\t\t\"1\"\n\t\"name\"\t\t\"EA Sports FC 27\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"EA Sports FC 27\"\n\t\"LastUpdated\"\t\t\"1791038343\"\n\t\"SizeOnDisk\"\t\t\"0\"\n\t\"buildid\"\t\t\"0\"\n}\n").unwrap();
        let dir = e.steamapps.join("common").join("EA Sports FC 27");
        fs::create_dir_all(&dir).unwrap();

        let p = prepare(&e, "4080220", "EA SPORTS FC 27", "fc27.exe").unwrap();
        assert!(p.exe_path.is_file());
        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "4080220"), Ok(true));
        assert!(!acf.exists(), "adopted legacy manifest is removed");
        assert!(!dir.exists(), "adopted legacy empty folder is removed");
        let _ = fs::remove_dir_all(&e.root);
    }


    /// Exactly what Steam rewrote the leftover FC 27 manifest to (observed on a real library).
    const STEAM_REWRITTEN: &str = "\"AppState\"\n{\n\t\"appid\"\t\t\"4080220\"\n\t\"Universe\"\t\t\"1\"\n\t\"LauncherPath\"\t\t\"C:\\Program Files (x86)\\Steam\\steam.exe\"\n\t\"name\"\t\t\"EA SPORTS FC™ 27\"\n\t\"StateFlags\"\t\t\"6\"\n\t\"installdir\"\t\t\"EA Sports FC 27\"\n\t\"LastUpdated\"\t\t\"1791038343\"\n\t\"LastPlayed\"\t\t\"0\"\n\t\"SizeOnDisk\"\t\t\"0\"\n\t\"StagingSize\"\t\t\"0\"\n\t\"buildid\"\t\t\"0\"\n\t\"LastOwner\"\t\t\"0\"\n\t\"DownloadType\"\t\t\"0\"\n\t\"AutoUpdateBehavior\"\t\t\"0\"\n\t\"AllowOtherDownloadsWhileRunning\"\t\t\"0\"\n\t\"ScheduledAutoUpdate\"\t\t\"1791150617\"\n\t\"InstalledDepots\"\n\t{\n\t}\n\t\"SharedDepots\"\n\t{\n\t\t\"228989\"\t\t\"228980\"\n\t}\n\t\"UserConfig\"\n\t{\n\t}\n\t\"MountedConfig\"\n\t{\n\t}\n}\n";

    #[test]
    fn adopts_an_empty_install_manifest_that_steam_rewrote() {
        let e = env("rewritten");
        let acf = e.steamapps.join("appmanifest_4080220.acf");
        fs::write(&acf, STEAM_REWRITTEN).unwrap();

        let p = prepare(&e, "4080220", "EA SPORTS FC 27", "fc27.exe").unwrap();
        // replaced by our manifest, which turns background updates off
        let text = fs::read_to_string(&acf).unwrap();
        assert!(text.contains("\"AutoUpdateBehavior\"\t\t\"1\""));
        assert!(text.contains("\"StateFlags\"\t\t\"4\""));
        assert!(p.exe_path.is_file());

        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "4080220"), Ok(true));
        assert!(!acf.exists());
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn never_adopts_real_or_pending_installs() {
        let e = env("pending");
        let acf = e.steamapps.join("appmanifest_9.acf");
        // queued download: has a target build
        let queued = STEAM_REWRITTEN.replace("4080220", "9").replace("\t\"LastOwner\"", "\t\"TargetBuildID\"\t\t\"123\"\n\t\"LastOwner\"");
        fs::write(&acf, &queued).unwrap();
        assert!(prepare(&e, "9", "Game", "g.exe").unwrap_err().contains("already has a manifest"));
        // real install: installed depots with manifests
        let real = STEAM_REWRITTEN.replace("4080220", "9").replace("\t\"InstalledDepots\"\n\t{\n\t}", "\t\"InstalledDepots\"\n\t{\n\t\t\"1\"\n\t\t{\n\t\t\t\"manifest\"\t\t\"55\"\n\t\t}\n\t}");
        fs::write(&acf, &real).unwrap();
        assert!(prepare(&e, "9", "Game", "g.exe").unwrap_err().contains("already has a manifest"));
        // real install: has a build id and size
        let sized = STEAM_REWRITTEN.replace("4080220", "9").replace("\"buildid\"\t\t\"0\"", "\"buildid\"\t\t\"2000\"").replace("\"SizeOnDisk\"\t\t\"0\"", "\"SizeOnDisk\"\t\t\"5000\"");
        fs::write(&acf, &sized).unwrap();
        assert!(prepare(&e, "9", "Game", "g.exe").unwrap_err().contains("already has a manifest"));
        assert_eq!(fs::read_to_string(&acf).unwrap(), sized, "manifest must be left untouched");
        let _ = fs::remove_dir_all(&e.root);
    }

    /// AION 2: Steam's launch path is absolute-looking and nested ("/Aion2/Binaries/Win64/AION2.exe"),
    /// and the install folder differs in case from the folder name inside the path.
    #[test]
    fn handles_a_nested_launch_path_with_a_leading_slash() {
        let json: serde_json::Value = serde_json::from_str(r#"{"data":{"3393110":{"config":{"installdir":"AION2","launch":{
            "0":{"arguments":"-steam","config":{"oslist":"windows"},"executable":"/Aion2/Binaries/Win64/AION2.exe"},
            "1":{"config":{"betakey":"aion2_dev_win_globaldist_gfn","oslist":"windows"},"executable":"/Aion2/Binaries/Win64/AION2.exe"}}}}}}"#).unwrap();
        let (dir, exe) = parse_steamcmd_info(&json, "3393110");
        assert_eq!(dir.as_deref(), Some("AION2"));
        let exe = exe.expect("a launch exe is found");
        assert_eq!(exe, "Aion2/Binaries/Win64/AION2.exe", "Steam's leading slash is dropped");

        let e = env("aion2");
        let p = prepare_fake(&e.steamapps, &e.registry, &e.runner, "3393110", "AION 2", &dir.unwrap(), &exe)
            .unwrap_or_else(|err| panic!("prepare failed for exe {:?}: {}", exe, err));
        assert!(p.exe_path.is_file(), "dummy exe must exist at {:?}", p.exe_path);
        assert!(p.exe_path.ends_with("AION2/Aion2/Binaries/Win64/AION2.exe"));
        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "3393110"), Ok(true));
        assert!(!e.steamapps.join("common").join("AION2").exists());
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn open_target_stays_inside_the_steam_library() {
        let e = env("open");
        let common = e.steamapps.join("common");

        // nothing created yet -> the library folder
        assert_eq!(resolve_open_target(&e.steamapps, "AION2", None), Ok(OpenTarget::Folder(common.clone())));

        // a created fake install -> its folder, and its exe when one is given
        let p = prepare_fake(&e.steamapps, &e.registry, &e.runner, "3393110", "AION 2", "AION2", "Aion2/Binaries/Win64/AION2.exe").unwrap();
        assert_eq!(resolve_open_target(&e.steamapps, "AION2", None), Ok(OpenTarget::Folder(common.join("AION2"))));
        let exe = p.exe_path.to_string_lossy().to_string();
        assert_eq!(resolve_open_target(&e.steamapps, "AION2", Some(&exe)), Ok(OpenTarget::Select(p.exe_path.clone())));

        // a file outside the library is ignored, not opened
        let outside = e.root.join("runner.exe").to_string_lossy().to_string();
        assert_eq!(resolve_open_target(&e.steamapps, "AION2", Some(&outside)), Ok(OpenTarget::Folder(common.join("AION2"))));

        // folder names cannot escape the library
        for bad in ["..", "../x", "a/b", "C:"] {
            assert!(resolve_open_target(&e.steamapps, bad, None).is_err(), "install dir {:?}", bad);
        }
        let _ = fs::remove_dir_all(&e.root);
    }

    /// Regression: titles that broke the card's generated names in a check over every Steam-linked game.
    #[test]
    fn accepts_long_titles_in_any_script() {
        let e = env("long");
        // 100 characters of Japanese is 300 bytes: the limit is on characters, not bytes
        let folder: String = "あ".repeat(100);
        assert!(folder.len() > 100 && folder.chars().count() == 100);
        let p = prepare_fake(&e.steamapps, &e.registry, &e.runner, "70300", "VVVVVV", &folder, "Game.exe").unwrap();
        assert!(p.exe_path.is_file());
        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "70300"), Ok(true));
        // 101 characters is still rejected
        assert!(prepare_fake(&e.steamapps, &e.registry, &e.runner, "70300", "VVVVVV", &"あ".repeat(101), "Game.exe").is_err());
        let _ = fs::remove_dir_all(&e.root);
    }

    // Real shape of Steam's data for AION 2 (3393110): one real depot, two shared Steamworks depots.
    const AION2_BUILD_JSON: &str = r#"{"data":{"3393110":{"depots":{
        "228989":{"config":{"oslist":"windows"},"depotfromapp":"228980","sharedinstall":"1"},
        "228990":{"config":{"oslist":"windows"},"depotfromapp":"228980","sharedinstall":"1"},
        "3393111":{"manifests":{"public":{"download":"86124054528","gid":"1699273097574069541","size":"87549550890"}}},
        "baselanguages":"english","branches":{"public":{"buildid":"25767555","timeupdated":"1791355218"}},"privatebranches":"1"}}}}"#;

    #[test]
    fn parses_the_build_and_depots_of_an_app() {
        let json: serde_json::Value = serde_json::from_str(AION2_BUILD_JSON).unwrap();
        let build = parse_steam_build(&json, "3393110").expect("a build");
        assert_eq!(build.build_id, "25767555");
        assert_eq!(build.depots, vec![SteamDepot { id: "3393111".into(), manifest: "1699273097574069541".into(), size: "87549550890".into() }]);
        assert_eq!(build.shared_depots, vec![("228989".to_string(), "228980".to_string()), ("228990".to_string(), "228980".to_string())]);
        assert_eq!(build.total_size(), 87_549_550_890);
        // unknown app, or no public build -> no build data (the minimal manifest is used instead)
        assert!(parse_steam_build(&json, "999").is_none());
        let none: serde_json::Value = serde_json::from_str(r#"{"data":{"1":{"depots":{"branches":{}}}}}"#).unwrap();
        assert!(parse_steam_build(&none, "1").is_none());
    }

    #[test]
    fn the_manifest_looks_like_a_finished_install_when_steam_data_is_known() {
        let e = env("rich");
        let json: serde_json::Value = serde_json::from_str(AION2_BUILD_JSON).unwrap();
        let build = parse_steam_build(&json, "3393110").unwrap();
        prepare_fake_with_build(&e.steamapps, &e.registry, &e.runner, "3393110", "AION 2", "AION2", "Aion2/Binaries/Win64/AION2.exe", Some(&build)).unwrap();
        let text = fs::read_to_string(e.steamapps.join("appmanifest_3393110.acf")).unwrap();

        for expected in [
            "\"appid\"\t\t\"3393110\"",
            "\"name\"\t\t\"AION 2\"",
            "\"installdir\"\t\t\"AION2\"",
            "\"StateFlags\"\t\t\"4\"",
            "\"buildid\"\t\t\"25767555\"",
            "\"TargetBuildID\"\t\t\"25767555\"",
            "\"SizeOnDisk\"\t\t\"87549550890\"",
            "\"AutoUpdateBehavior\"\t\t\"1\"",
            "\"3393111\"\n\t\t{\n\t\t\t\"manifest\"\t\t\"1699273097574069541\"\n\t\t\t\"size\"\t\t\"87549550890\"",
            "\"228989\"\t\t\"228980\"",
            "\"LauncherPath\"",
            "steam.exe",
            "\"UserConfig\"",
        ] {
            assert!(text.contains(expected), "manifest is missing {:?}\n{}", expected, text);
        }
        // braces balance and every key has a value: Steam's parser is unforgiving
        assert_eq!(text.matches('{').count(), text.matches('}').count());
        assert!(text.trim_start().starts_with("\"AppState\""));
        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "3393110"), Ok(true));
        assert!(!e.steamapps.join("appmanifest_3393110.acf").exists());
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn without_steam_data_the_manifest_is_minimal_but_valid() {
        let e = env("thin");
        prepare(&e, "4080220", "EA SPORTS FC 27", "fc27.exe").unwrap();
        let text = fs::read_to_string(e.steamapps.join("appmanifest_4080220.acf")).unwrap();
        assert!(text.contains("\"buildid\"\t\t\"0\"") && text.contains("\"SizeOnDisk\"\t\t\"0\""));
        assert!(!text.contains("TargetBuildID") && !text.contains("\"manifest\""));
        assert_eq!(text.matches('{').count(), text.matches('}').count());
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn launching_again_replaces_a_manifest_we_made_earlier() {
        let e = env("relaunch");
        // first launch (for example from an older version) wrote a minimal manifest and was never cleaned up
        prepare_fake(&e.steamapps, &e.registry, &e.runner, "3393110", "AION 2", "AION2", "Aion2/Binaries/Win64/AION2.exe").unwrap();
        let acf = e.steamapps.join("appmanifest_3393110.acf");
        assert!(fs::read_to_string(&acf).unwrap().contains("\"buildid\"\t\t\"0\""));

        let json: serde_json::Value = serde_json::from_str(AION2_BUILD_JSON).unwrap();
        let build = parse_steam_build(&json, "3393110").unwrap();
        prepare_fake_with_build(&e.steamapps, &e.registry, &e.runner, "3393110", "AION 2", "AION2", "Aion2/Binaries/Win64/AION2.exe", Some(&build)).unwrap();
        assert!(fs::read_to_string(&acf).unwrap().contains("\"buildid\"\t\t\"25767555\""), "the stale manifest must be replaced");

        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "3393110"), Ok(true));
        assert!(!acf.exists());
        let _ = fs::remove_dir_all(&e.root);
    }
    #[test]
    fn rejects_path_tricks() {
        let e = env("paths");
        for bad in ["..", "a/b", "a\\b", "C:", "x*y", "", "name."] {
            assert!(prepare(&e, "5", bad, "x.exe").is_err(), "install dir {:?}", bad);
        }
        for bad in ["../evil.exe", "a/../../evil.exe", "C:/evil.exe", "noext", "x.exe/..", ""] {
            assert!(prepare(&e, "5", "Game", bad).is_err(), "exe {:?}", bad);
        }
        assert!(prepare(&e, "abc", "Game", "x.exe").is_err());
        assert!(prepare(&e, "../1", "Game", "x.exe").is_err());
        assert!(!e.steamapps.join("common").join("Game").exists());
        // nested exe paths are fine and cleaned up
        prepare(&e, "6", "Game", "Shipping/Product/Game.exe").unwrap();
        assert_eq!(cleanup_fake(&e.steamapps, &e.registry, "6"), Ok(true));
        assert!(!e.steamapps.join("common").join("Game").exists());
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn cleanup_ignores_tampered_registry() {
        let e = env("tamper");
        let victim = e.root.join("important.txt");
        fs::write(&victim, b"keep me").unwrap();
        let entries = vec![SteamFake {
            steam_id: "7".into(),
            acf_path: e.steamapps.join("appmanifest_7.acf"),
            acf_created: true,
            exe_path: victim.clone(),
            created_dirs: vec![],
        }];
        save_registry(&e.registry, &entries).unwrap();
        assert!(cleanup_fake(&e.steamapps, &e.registry, "7").is_err());
        assert!(victim.exists(), "must never delete outside the Steam library");
        let _ = fs::remove_dir_all(&e.root);
    }

    #[test]
    fn parses_steamcmd_launch_info() {
        // FC 27: launcher URL only -> no exe; Tokon: first non-beta windows exe
        let fc27: serde_json::Value = serde_json::from_str(r#"{"data":{"4080220":{"config":{"installdir":"EA SPORTS FC 27","launch":{"0":{"executable":"steam2ea://launchgame/4080220?platform=steam&theme=fc27"}}}}}}"#).unwrap();
        assert_eq!(parse_steamcmd_info(&fc27, "4080220"), (Some("EA SPORTS FC 27".into()), None));

        let tokon: serde_json::Value = serde_json::from_str(r#"{"data":{"3787240":{"config":{"installdir":"MTFS","launch":{
            "1":{"config":{"betakey":"mtfs_debuggame"},"executable":"REDSteam.exe"},
            "0":{"executable":"start_protected_game.exe"},
            "2":{"config":{"oslist":"linux"},"executable":"run.sh"}}}}}}"#).unwrap();
        assert_eq!(parse_steamcmd_info(&tokon, "3787240"), (Some("MTFS".into()), Some("start_protected_game.exe".into())));
        assert_eq!(parse_steamcmd_info(&tokon, "999"), (None, None));
    }
}
