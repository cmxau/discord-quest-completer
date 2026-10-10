import { randomString } from '@/utils/random-string';
import type { Game, GameExecutable } from '@/types/types';

// The list of games the user added is kept in the webview's local storage so it survives restarts.
// Only what identifies a game, plus the user's own marks (favorite, last used), is stored. Runtime state (running, installed) is never saved, and the
// Discord list is the source of truth for everything else: saved entries are refreshed from it.
const STORAGE_KEY = 'discord-quest-completer.games.v1';

function cloneExecutable(executable: GameExecutable): GameExecutable {
    return {
        name: executable.name,
        os: executable.os,
        is_launcher: !!executable.is_launcher,
        ...(executable.is_auto_generated ? { is_auto_generated: true } : {}),
    };
}

/** A clean copy of a game with only the fields that are saved (also used when adding a game). */
export function cloneGame(game: Game): Game {
    return {
        id: String(game.id),
        name: game.name,
        aliases: [...(game.aliases ?? [])],
        themes: [...(game.themes ?? [])],
        third_party_skus: (game.third_party_skus ?? []).map(sku => ({ distributor: sku.distributor, id: sku.id })),
        executables: (game.executables ?? []).map(cloneExecutable),
        ...(game.favorite ? { favorite: true } : {}),
        ...(typeof game.last_used === 'number' && Number.isFinite(game.last_used) ? { last_used: game.last_used } : {}),
    };
}

/** The saved form of the list. Changes only when something worth saving changes. */
export function serializeGames(games: Game[]): string {
    return JSON.stringify(games.map(cloneGame));
}

type Storage = Pick<globalThis.Storage, 'getItem' | 'setItem'>;

function defaultStorage(): Storage | null {
    try {
        return globalThis.localStorage ?? null;
    } catch {
        return null; // storage can be blocked (private window, cleared site data)
    }
}

export function saveGames(games: Game[], storage: Storage | null = defaultStorage()): void {
    try {
        storage?.setItem(STORAGE_KEY, serializeGames(games));
    } catch {
        // out of space or blocked: the list just won't persist this time
    }
}

/** The saved games, each with a fresh `uid`. Anything unreadable is ignored instead of throwing. */
export function loadSavedGames(storage: Storage | null = defaultStorage()): Game[] {
    try {
        const parsed: unknown = JSON.parse(storage?.getItem(STORAGE_KEY) ?? '[]');
        if (!Array.isArray(parsed)) {
            return [];
        }
        const seen = new Set<string>();
        const games: Game[] = [];
        for (const item of parsed) {
            if (!item || typeof item.id !== 'string' || typeof item.name !== 'string' || !Array.isArray(item.executables)) {
                continue;
            }
            if (seen.has(item.id)) {
                continue;
            }
            seen.add(item.id);
            games.push({ uid: randomString(), ...cloneGame(item) });
        }
        return games;
    } catch {
        return [];
    }
}

/**
 * Update saved games in place from a freshly loaded Discord list: names, links and executables
 * follow Discord's data. A game that is running keeps its executables so its state isn't disturbed,
 * and a game Discord no longer lists keeps what was saved.
 */
export function refreshSavedGames(saved: Game[], fresh: Game[]): void {
    const byId = new Map(fresh.map(game => [String(game.id), game]));
    for (const game of saved) {
        const latest = byId.get(game.id);
        if (!latest) {
            continue;
        }
        const clean = cloneGame(latest);
        game.name = clean.name;
        game.aliases = clean.aliases;
        game.themes = clean.themes;
        game.third_party_skus = clean.third_party_skus;
        const busy = game.is_running || !!game.steam_exe || game.executables.some(exe => exe.is_running);
        if (!busy) {
            game.executables = clean.executables;
        }
    }
}
