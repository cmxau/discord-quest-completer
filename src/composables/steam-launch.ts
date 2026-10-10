import { invoke } from '@tauri-apps/api/core';
import { toPascalCase } from '@/utils/fallback-executable';
import type { Game } from '@/types/types';

export interface SteamInfo {
    steam_found: boolean;
    installdir: string | null;
    exe: string | null;
    /** Steam's data could not be fetched (network, rate limit); the fields above are empty for that reason. */
    lookup_failed: boolean;
}

/** What `launch_steam_game` reports: the exe file name and where the dummy exe was created. */
export interface LaunchedSteamGame {
    file_name: string;
    exe_path: string;
    /** True when the manifest was filled from Steam's build data, false when it is a minimal one. */
    manifest_from_steam: boolean;
    /** Where the Steam manifest was written. */
    manifest_path: string;
}

const infoCache = new Map<string, SteamInfo>();

/** The Steam app id Discord links to this game, if any. */
export function getSteamId(game: Game | null): string | null {
    // Discord's data has a few malformed ids ("1335200 ", "40390\t", "null"): clean up what can be
    // cleaned and ignore the rest, since a Steam app id is always digits.
    for (const sku of game?.third_party_skus ?? []) {
        const id = sku.distributor === 'steam' ? String(sku.id ?? '').trim() : '';
        if (/^\d+$/.test(id)) {
            return id;
        }
    }
    return null;
}

export async function fetchSteamInfo(steamId: string): Promise<SteamInfo> {
    const cached = infoCache.get(steamId);
    if (cached) {
        return cached;
    }
    const info = await invoke<SteamInfo>('steam_game_info', { steam_id: steamId });
    // Never remember a failed lookup, or one hiccup would leave the card empty until the app restarts.
    if (!info.lookup_failed) {
        infoCache.set(steamId, info);
    }
    return info;
}

/**
 * A folder name Windows and the backend accept: the characters they reject are dropped, and
 * trailing dots and spaces are trimmed. Falls back to "Game" so there is always a usable name.
 */
export function defaultInstallFolder(gameName: string): string {
    const cleaned = gameName
        // eslint-disable-next-line no-control-regex
        .replace(/[<>:"/\\|?*\u0000-\u001f]/g, ' ')
        .replace(/\s+/g, ' ')
        .trim();
    // Array.from walks whole characters, so a long title in any script (or an emoji) is cut at 100
    // characters without being split in the middle of one. Windows rejects trailing dots and spaces.
    const limited = Array.from(cleaned).slice(0, 100).join('').trim().replace(/[. ]+$/, '');
    return limited || 'Game';
}

/**
 * The exe name to use. Steam's launch config is used when it names a real .exe. Some games (for
 * example EA titles) only launch through a URL, and Steam has no data for others, so fall back to a
 * name derived from the folder or the title ("EA SPORTS FC 27" -> fc27.exe, "Shift at Midnight" ->
 * ShiftAtMidnight.exe). It is never empty, so Launch always has something to create; when it is a
 * fallback, `guessed` is true and the user can edit it.
 */
export function guessExeName(
    installdir: string | null,
    steamExe: string | null,
    gameName: string,
): { exe: string; guessed: boolean } {
    if (steamExe) {
        return { exe: steamExe, guessed: false };
    }
    const source = installdir ?? gameName;
    const sequel = source.match(/([A-Za-z]{2,4})\s*(\d{2,4})$/);
    if (sequel) {
        return { exe: `${sequel[1].toLowerCase()}${sequel[2]}.exe`, guessed: true };
    }
    // The backend rejects names over 100 characters, and some titles are very long.
    const base = (toPascalCase(source) || toPascalCase(gameName)).slice(0, 80);
    return { exe: `${base || 'Game'}.exe`, guessed: true };
}

/** Create the fake Steam install and start the dummy exe. Marks the game as running. */
export async function launchSteamGame(game: Game, steamId: string, installDir: string, exeName: string): Promise<LaunchedSteamGame> {
    const launched = await invoke<LaunchedSteamGame>('launch_steam_game', {
        steam_id: steamId,
        name: game.name,
        install_dir: installDir,
        exe_name: exeName,
    });
    game.steam_exe = launched.file_name;
    game.is_running = true;
    return launched;
}

/**
 * Stop the dummy process. Everything created in the Steam library for this game is removed too,
 * unless the user keeps Steam entries. Resolves to whether the entry was removed.
 */
export async function stopSteamGame(game: Game): Promise<boolean> {
    const steamId = getSteamId(game);
    const exe = game.steam_exe;
    if (!steamId || !exe) {
        return false;
    }
    try {
        return await invoke<boolean>('stop_steam_game', { steam_id: steamId, exe_filename: exe });
    } finally {
        game.steam_exe = undefined;
        game.is_running = game.executables.some(executable => executable.is_running);
    }
}
