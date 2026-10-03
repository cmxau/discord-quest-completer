# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A **Windows-only** Tauri 2 desktop app (Vue 3 + TypeScript + Tailwind 4 frontend, Rust backend) that completes Discord Quests / shows Rich Presence for games without installing them. It creates a fake "game" executable (a copy of a tiny runner binary, placed at the path Discord detects) and launches it so Discord sees the game as running. It is built on `markterence/discord-quest-completer` (MIT); the original copyright notice in `LICENSE` must be kept.

## Commands

Package manager is pnpm; Rust toolchain and Tauri prerequisites required.

```bash
pnpm install
pnpm sync:runner            # builds ./runner and copies quest-runner.exe into src-tauri/resources/
pnpm tauri dev              # run app (starts vite on :1420 via beforeDevCommand)
pnpm build                  # vue-tsc --noEmit && vite build (frontend typecheck + build)
cargo test --lib --manifest-path src-tauri/Cargo.toml   # Rust unit tests
pnpm tauri build            # full bundle
```

The runner must be built and copied to `src-tauri/resources/` **before** `tauri dev`/`build`, and the frontend must be built (`dist/`) before compiling the Rust crate in CI. There is no frontend test or lint setup.

## Architecture

- **`src/`** – Vue frontend. `pages/HomeView.vue` is the main UI (sidebar of added games + actions panel); `pages/Diagnostics.vue` is the log/RPC test page, opened from the header. `components/MainLayout.vue` is the header (diagnostics icon, theme toggle, list refresh); `components/SteamLaunch.vue` is the Steam library card. State lives in `composables/`: `app-state.ts` (shared), `fetch-gamelist.ts` (Discord API -> GitHub Pages mirror -> bundled `assets/gamelist.json`, then merges `assets/custom-games.json` and adds generated fallback executables from `utils/fallback-executable.ts`), `running-state.ts` (which games are running, synced with real processes every ~1.5s), `steam-launch.ts` (Steam commands).
- **`src-tauri/`** – Rust backend (lib `tauri_app_lib`). `src/lib.rs` holds the Tauri commands: `create_fake_game`, `run_background_process`, `stop_process` (taskkill), `connect_to_discord_rpc_3` (Rich Presence via `rpc.rs` / `runner.rs` using `discord-sdk`), the game-list fetchers, and `steam_game_info` / `launch_steam_game` / `stop_steam_game`. `src/steam.rs` is the Steam library logic (fake manifest + exe under `steamapps/common`, a registry of what was created in the app data folder, cleanup, path validation) and is unit-tested against temporary folders. `src/processes.rs` keeps handles of spawned dummy processes so `running_exes` can report which are alive. Frontend calls commands with `invoke` using snake_case args; Discord app ids are passed as strings (JS numbers lose precision).
- **`runner/`** – the dummy game window: a `no_std`/`no_main` Rust crate whose real entry point is `main.cpp` (a small Win32 window, no C runtime) via `/ENTRY` set in `build.rs`. It takes `--title`. Built to `quest-runner.exe` and bundled as `data/quest-runner.exe` (see `tauri.windows.conf.json`).
- **`docs/how-it-works.md`** explains the detection approach and the Steam safeguards.

## Notes

- Versions in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` should be kept in sync.
- The app writes game folders next to its executable, so it needs a writable install location. Steam fakes are recorded in `%APPDATA%\io.github.cmxau.discordquestcompleter\steam_fakes.json`.
- The app icon is drawn by `scripts/make-icon.ps1` into `src-tauri/icons/app-icon-source.png`; regenerate all sizes with `pnpm tauri icon src-tauri/icons/app-icon-source.png` (then delete the generated `android/` and `ios/` folders).
- `.github/workflows/` has CI, a tag-triggered release, and a daily Pages job that publishes the game-list mirror.
