import { invoke } from '@tauri-apps/api/core';
import { toPascalCase } from '@/utils/fallback-executable';
import type { Game } from '@/types/types';

export interface SteamInfo {
    steam_found: boolean;
    installdir: string | null;
    exe: string | null;
}

const infoCache = new Map<string, SteamInfo>();

/** The Steam app id Discord links to this game, if any. */
export function getSteamId(game: Game | null): string | null {
    return game?.third_party_skus?.find(sku => sku.distributor === 'steam')?.id ?? null;
}

export async function fetchSteamInfo(steamId: string): Promise<SteamInfo> {
    const cached = infoCache.get(steamId);
    if (cached) {
        return cached;
    }
    const info = await invoke<SteamInfo>('steam_game_info', { steam_id: steamId });
    infoCache.set(steamId, info);
    return info;
}

/**
 * Best guess for the exe name. Steam's launch config is used when it names a real .exe; some games
 * (e.g. EA titles) only launch through a URL, so fall back to the pattern Discord uses for sibling
 * games ("EA SPORTS FC 27" -> fc27.exe). The result is only a guess and the user can edit it.
 */
export function guessExeName(installdir: string | null, steamExe: string | null): { exe: string; guessed: boolean } {
    if (steamExe) {
        return { exe: steamExe, guessed: false };
    }
    if (!installdir) {
        return { exe: '', guessed: true };
    }
    const sequel = installdir.match(/([A-Za-z]{2,4})\s*(\d{2,4})$/);
    if (sequel) {
        return { exe: `${sequel[1].toLowerCase()}${sequel[2]}.exe`, guessed: true };
    }
    const base = toPascalCase(installdir);
    return { exe: base ? `${base}.exe` : '', guessed: true };
}

/** Create the fake Steam install and start the dummy exe. Marks the game as running. */
export async function launchSteamGame(game: Game, steamId: string, installDir: string, exeName: string) {
    const fileName = await invoke<string>('launch_steam_game', {
        steam_id: steamId,
        name: game.name,
        install_dir: installDir,
        exe_name: exeName,
    });
    game.steam_exe = fileName;
    game.is_running = true;
}

/** Stop the dummy process and remove everything created in the Steam library for this game. */
export async function stopSteamGame(game: Game) {
    const steamId = getSteamId(game);
    const exe = game.steam_exe;
    if (!steamId || !exe) {
        return;
    }
    try {
        await invoke('stop_steam_game', { steam_id: steamId, exe_filename: exe });
    } finally {
        game.steam_exe = undefined;
        game.is_running = game.executables.some(executable => executable.is_running);
    }
}
