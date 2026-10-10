// The shape of the app's settings and how saved settings are cleaned up. No Vue or Tauri in here,
// so it can be checked on its own.

export type SortOrder = 'added' | 'name' | 'recent';

export interface Settings {
    /** Stop a game on its own after a while, so a dummy is never left running by mistake. */
    autoStopEnabled: boolean;
    /** Minutes a game runs before it is stopped (a quest usually needs 15, plus a margin). */
    autoStopMinutes: number;
    /** Per-game minutes by Discord app id. 0 means "never stop this one". */
    gameMinutes: Record<string, number>;
    /** Quitting stops the dummy games (and removes their Steam entries, unless those are kept). */
    stopOnExit: boolean;
    /** Steam entries stay in the Steam library after a game stops, until the user removes them. */
    keepSteamEntries: boolean;
    /** The close button hides the window to the tray instead of quitting. */
    closeToTray: boolean;
    /** When Windows starts the app, keep it in the tray instead of opening the window. */
    startMinimized: boolean;
    checkDiscord: boolean;
    checkUpdates: boolean;
    sortOrder: SortOrder;
}

export const MAX_MINUTES = 720;

export const DEFAULT_SETTINGS: Settings = {
    autoStopEnabled: true,
    autoStopMinutes: 20,
    gameMinutes: {},
    stopOnExit: true,
    keepSteamEntries: false,
    closeToTray: false,
    startMinimized: true,
    checkDiscord: true,
    checkUpdates: true,
    sortOrder: 'added',
};

function bool(value: unknown, fallback: boolean): boolean {
    return typeof value === 'boolean' ? value : fallback;
}

function minutes(value: unknown, min: number): number | null {
    return typeof value === 'number' && Number.isFinite(value)
        ? Math.min(MAX_MINUTES, Math.max(min, Math.round(value)))
        : null;
}

/** Settings from whatever was saved: anything missing, wrongly typed or out of range gets its default. */
export function normalizeSettings(raw: unknown): Settings {
    const source = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
    const gameMinutes: Record<string, number> = {};
    if (source.gameMinutes && typeof source.gameMinutes === 'object' && !Array.isArray(source.gameMinutes)) {
        for (const [id, value] of Object.entries(source.gameMinutes as Record<string, unknown>)) {
            const clean = minutes(value, 0);
            if (/^\d+$/.test(id) && clean !== null) {
                gameMinutes[id] = clean;
            }
        }
    }
    return {
        autoStopEnabled: bool(source.autoStopEnabled, DEFAULT_SETTINGS.autoStopEnabled),
        autoStopMinutes: minutes(source.autoStopMinutes, 1) ?? DEFAULT_SETTINGS.autoStopMinutes,
        gameMinutes,
        stopOnExit: bool(source.stopOnExit, DEFAULT_SETTINGS.stopOnExit),
        keepSteamEntries: bool(source.keepSteamEntries, DEFAULT_SETTINGS.keepSteamEntries),
        closeToTray: bool(source.closeToTray, DEFAULT_SETTINGS.closeToTray),
        startMinimized: bool(source.startMinimized, DEFAULT_SETTINGS.startMinimized),
        checkDiscord: bool(source.checkDiscord, DEFAULT_SETTINGS.checkDiscord),
        checkUpdates: bool(source.checkUpdates, DEFAULT_SETTINGS.checkUpdates),
        sortOrder: source.sortOrder === 'name' || source.sortOrder === 'recent' ? source.sortOrder : 'added',
    };
}

/** How long this game may run before it is stopped: 0 means never. */
export function autoStopMinutesFor(settings: Settings, gameId: string): number {
    if (!settings.autoStopEnabled) {
        return 0;
    }
    return settings.gameMinutes[gameId] ?? settings.autoStopMinutes;
}
