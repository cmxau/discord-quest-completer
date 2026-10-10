import { ref, type Ref } from 'vue';
import { useIntervalFn } from '@vueuse/core';
import { autoStopMinutesFor, useSettings } from '@/composables/settings';
import { useGlobalState } from '@/composables/app-state';
import type { Game } from '@/types/types';

/**
 * Times how long each game has been running: records when it was last used and stops it by itself
 * once its auto-stop time is up, so a dummy is never left running by mistake.
 */
export function useGameSessions(gameList: Ref<Game[]>, stopGame: (game: Game) => Promise<void>) {
    const { settings } = useSettings();
    const { addLog } = useGlobalState();

    const startedAt = ref<Record<string, number>>({});
    const now = ref(Date.now());
    const stopping = new Set<string>();
    const gaveUp = new Set<string>(); // stopping failed: don't retry every second

    /** Milliseconds this game may run, or 0 for no limit. */
    function limitMs(game: Game): number {
        return autoStopMinutesFor(settings, game.id) * 60_000;
    }

    /** Milliseconds until this game is stopped, or null when it isn't running or has no limit. */
    function remainingMs(game: Game): number | null {
        const started = game.uid ? startedAt.value[game.uid] : undefined;
        const limit = limitMs(game);
        if (started === undefined || !game.is_running || limit === 0) {
            return null;
        }
        return Math.max(0, started + limit - now.value);
    }

    async function tick() {
        now.value = Date.now();
        for (const game of gameList.value) {
            const uid = game.uid;
            if (!uid) {
                continue;
            }
            if (!game.is_running) {
                delete startedAt.value[uid];
                gaveUp.delete(uid);
                continue;
            }
            if (startedAt.value[uid] === undefined) {
                startedAt.value[uid] = now.value;
                game.last_used = now.value;
            }
            const left = remainingMs(game);
            if (left === 0 && !stopping.has(uid) && !gaveUp.has(uid)) {
                stopping.add(uid);
                const minutes = Math.round((now.value - startedAt.value[uid]) / 60_000);
                addLog('info', `Auto-stop: stopping ${game.name} after ${minutes} min`);
                try {
                    await stopGame(game);
                } catch (error) {
                    gaveUp.add(uid);
                    addLog('error', `Auto-stop could not stop ${game.name}: ${error}`);
                } finally {
                    stopping.delete(uid);
                }
            }
        }
    }
    useIntervalFn(tick, 1000, { immediateCallback: true });

    return { now, remainingMs };
}
