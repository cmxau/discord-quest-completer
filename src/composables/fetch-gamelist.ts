import { Game } from '@/types/types';
import { tryOnMounted, useAsyncState } from '@vueuse/core';
import { ref, watch } from 'vue';
import { message } from '@tauri-apps/plugin-dialog'; 
import { invoke } from '@tauri-apps/api/core';
import { useGlobalState } from './app-state';
import customGamesJson from '../assets/custom-games.json';
import { withFallbackExecutable } from '@/utils/fallback-executable';

export function useFetchGameList() {
    const { addLog } = useGlobalState();
    async function fetchGameListGHMirror() {
        addLog('Fetching game list from GitHub mirror...'); 
        const response = await invoke('fetch_gamelist_gh_mirror');
        return response as Game[] | unknown[] | undefined;
    }
    async function fetchGameListFromDiscord (){
        addLog('Fetching game list directly from discord...'); 
        const response = await invoke('fetch_gamelist_from_discord');
        return response as Game[] | unknown[] | undefined;
    };


    const { 
        state: gameListGHMirror,
        error: errorGH,
        isReady: isReadyGH,
        execute: executeGH,
        isLoading: isLoadingGH
    } = useAsyncState<Game[] | unknown[] | undefined>(fetchGameListGHMirror, [], {
            immediate: false,
            resetOnExecute: true,
        });
    const { 
        state: gameListFromDiscord, 
        error: errorDiscord,
        isReady: isReadyDiscord,
        execute: executeDiscord,
        isLoading: isLoadingDiscord
    } = useAsyncState(fetchGameListFromDiscord, [], {
        immediate: false,
        resetOnExecute: true,
    });
    const { 
        state: bundledGameList,
        error: errorBundled,
        isReady: isReadyBundled,
        execute: executeBundled,
        isLoading: isLoadingBundled
    } = useAsyncState(() => {
        const result = import('../assets/gamelist.json').then(res=>res.default);
        addLog('Fetching bundled game list for fallback...');
        return result;
    }, [], {
        immediate: false,
        resetOnExecute: true,
    });

    const fetchError = ref<string | null>(null);

    const gameDB = ref<Game[]>([]);

    const allFetchDone = ref(false);

    function isValidGameList(data: any): boolean {
        return Array.isArray(data) && data[0] && 'aliases' in data[0] && 'name' in data[0] && 'executables' in data[0];
    }

    watch(() => isReadyGH.value, async (newVal) => {
        addLog('debug','isReadyGH: ' + newVal); 
    });

    watch(() => isReadyDiscord.value, async (newVal) => {
        addLog('debug','isReadyDiscord: ' + newVal);
    })
    
    watch(() => isReadyBundled.value, async (newVal) => {
        addLog('debug','isReadyBundled: ' + newVal); 
    });

    let timeoutId: ReturnType<typeof setTimeout> | null = null;
    async function fetchGameList() {
        allFetchDone.value = false;
        fetchError.value = null;
        addLog('Fetching game list...');

        // Priority: Discord API (most up to date) -> GitHub mirror -> bundled JSON (last resort).
        let source: Game[] | null = null;

        try {
            await executeDiscord();
        } catch {
            addLog('error', 'Error executing fetch for Discord game list.');
        }
        if (!errorDiscord.value && isValidGameList(gameListFromDiscord.value)) {
            source = gameListFromDiscord.value as Game[];
            addLog('Using game list from Discord. ' + source.length + ' entries.');
        } else {
            addLog('error', 'Error fetching game list from Discord, trying GitHub mirror');

            try {
                await executeGH();
            } catch {
                addLog('error', 'Error executing fetch for GitHub mirror game list.');
            }
            if (!errorGH.value && isValidGameList(gameListGHMirror.value)) {
                source = gameListGHMirror.value as Game[];
                addLog('Using game list from GitHub mirror. ' + source.length + ' entries.');
            } else {
                addLog('error', 'Error fetching game list from GitHub mirror, using bundled list');
            }
        }

        if (!source) {
            try {
                await executeBundled();
            } catch {
                addLog('error', 'Error executing fetch for bundled game list.');
            }
            if (!errorBundled.value && isValidGameList(bundledGameList.value)) {
                source = bundledGameList.value as Game[];
                addLog('Using bundled game list as fallback. ' + source.length + ' entries.');
                fetchError.value = 'Could not reach Discord or the GitHub mirror, using the bundled list.';
            } else {
                source = [];
                fetchError.value = 'Error fetching the game list.';
                addLog('error', 'Error fetching bundled game list');
            }
        }

        if (fetchError.value) {
            await message('There was an error fetching the latest game list. ' + fetchError.value, {
                title: 'Game List Fetch Error',
                kind: 'error',
                buttons: {
                    ok: 'OK'
                }
            });
        }

        gameDB.value = source;

        // Merge locally maintained entries (e.g. newly launched games not yet in Discord's list).
        // Custom entries win over fetched ones with the same id.
        const customGames = customGamesJson as Game[];
        if (customGames.length > 0) {
            const customIds = new Set(customGames.map(g => g.id));
            gameDB.value = [...gameDB.value.filter(g => !customIds.has(g.id)), ...customGames];
            addLog('Merged ' + customGames.length + ' custom game(s).');
        }

        // Games with no registered executables get a generated one so they can still be launched.
        gameDB.value = gameDB.value.map(withFallbackExecutable);

        // Set a timeout to delay setting allFetchDone to true, to allow UI to update.
      
        timeoutId = setTimeout(() => {
            allFetchDone.value = true;
        }, 1800);

    }

    watch(allFetchDone, (newVal) => {
        if (newVal && timeoutId) {
            clearTimeout(timeoutId);
        }
    });

    tryOnMounted(async () => {
        await fetchGameList();
    });


    return {
        gameListGHMirror,
        gameListFromDiscord,
        bundledGameList,
        fetchError,
        isReadyGH,
        isReadyDiscord,
        isReadyBundled,
        gameDB,
        fetchGameList,
        isLoadingGH,
        isLoadingDiscord,
        isLoadingBundled,
        allFetchDone
    }
}