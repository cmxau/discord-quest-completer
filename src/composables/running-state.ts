import { computed, ref, type Ref } from 'vue';
import { useIntervalFn } from '@vueuse/core';
import { invoke } from '@tauri-apps/api/core';
import { useGlobalState } from '@/composables/app-state';
import { stopSteamGame } from '@/composables/steam-launch';
import { executableFileName } from '@/utils/executable-path';
import type { Game } from '@/types/types';

const SYNC_INTERVAL_MS = 1500;

/**
 * Tracks which games are running and keeps that in step with reality.
 *
 * A game counts as running while it has a Steam-library launch, a running executable, or the Rich
 * Presence session. The backend reports which dummy processes are still alive, so a game closed from
 * its own window (or Task Manager) stops showing as running.
 */
export function useRunningState(gameList: Ref<Game[]>) {
    const { addLog } = useGlobalState();

    // The game "playing" through Rich Presence, if any. It has no process, so it is tracked separately.
    const rpcGameUid = ref<string | null>(null);

    const playingGames = computed(() => gameList.value.filter(game => game.is_running));

    function recomputeRunning(game: Game) {
        game.is_running = !!game.steam_exe
            || game.executables.some(exe => exe.is_running)
            || game.uid === rpcGameUid.value;
    }

    let isSyncing = false;
    async function syncRunningState() {
        if (isSyncing || !gameList.value.some(game => game.is_running && game.uid !== rpcGameUid.value)) {
            return; // nothing launched from here is running, so there is nothing to check
        }
        isSyncing = true;
        try {
            let alive: Set<string>;
            try {
                alive = new Set(await invoke<string[]>('running_exes'));
            } catch {
                return;
            }
            for (const game of gameList.value) {
                let changed = false;
                for (const executable of game.executables) {
                    const file = executableFileName(executable.name).toLowerCase();
                    if (executable.is_running && !alive.has(file)) {
                        executable.is_running = false;
                        changed = true;
                        addLog('info', `${game.name} (${file}) exited`);
                    }
                }
                if (game.steam_exe && !alive.has(game.steam_exe.toLowerCase())) {
                    addLog('info', `${game.name} (${game.steam_exe}) exited, removing its Steam library entry`);
                    try {
                        await stopSteamGame(game); // also clears game.steam_exe
                    } catch (error) {
                        addLog('error', `Failed to clean up Steam library entry for ${game.name}: ${error}`);
                    }
                    changed = true;
                }
                if (changed) {
                    recomputeRunning(game);
                }
            }
        } finally {
            isSyncing = false;
        }
    }
    useIntervalFn(syncRunningState, SYNC_INTERVAL_MS);

    return { rpcGameUid, playingGames, recomputeRunning };
}
