<script setup lang="ts">
import { ref, computed, useTemplateRef, shallowRef, provide, watch } from 'vue';
import { onClickOutside, refDebounced, useEventListener } from '@vueuse/core';
import { useFuse } from '@vueuse/integrations/useFuse'
import { invoke } from '@tauri-apps/api/core';
import { randomString } from '@/utils/random-string';
import { GameActionsProvider, GameExecutable, type Game } from '@/types/types';
import IconVerified from '@/components/IconVerified.vue';
import GameExecutables from '@/components/GameExecutables.vue';
import SteamLaunch from '@/components/SteamLaunch.vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { path } from '@tauri-apps/api';
import {
    defaultInstallFolder, fetchSteamInfo, getSteamId, guessExeName, launchSteamGame, stopSteamGame,
} from '@/composables/steam-launch';
import { executableDirectory, executableFileName, isValidExecutablePath } from '@/utils/executable-path';
import { formatAgo, formatCountdown } from '@/utils/time';
import { useSettings } from '@/composables/settings';
import { useGameSessions } from '@/composables/game-sessions';
import AutoStopCard from '@/components/AutoStopCard.vue';
import { useRunningState } from '@/composables/running-state';
import { cloneGame, loadSavedGames, refreshSavedGames, saveGames, serializeGames } from '@/composables/saved-games';
import { GameActionsKey, getCurrentOS } from '@/constants/constants';
import { emit, listen } from '@tauri-apps/api/event';
import { useFetchGameList } from '@/composables/fetch-gamelist';
import { UseFuseOptions } from '@vueuse/integrations';
import Fuse from 'fuse.js';
import { Pages, useGlobalState } from '@/composables/app-state';
import TimedNotification from '@/components/TimedNotification.vue';

type DialogKey =
    'none' | 
    'rpc_message_1'|
    'no_game_selected';;


const {
    gameDB,
    isLoadingBundled,
    isLoadingDiscord,
    isLoadingGH,
    fetchGameList,
    isReadyGH,
    isReadyBundled,
    isReadyDiscord,
    allFetchDone,
} = useFetchGameList()
const { addLog, refreshRequest, isGameListLoading, selectedGameLabel, page } = useGlobalState();
const { settings } = useSettings();
watch(refreshRequest, () => fetchGameList());
watch(
    () => isLoadingGH.value || isLoadingDiscord.value || isLoadingBundled.value,
    (loading) => { isGameListLoading.value = loading; },
    { immediate: true },
);
const shouldShowNotificationContainer = computed(() => {
    return isLoadingGH.value || isLoadingDiscord.value || isLoadingBundled.value ||
           (isReadyGH.value || isReadyDiscord.value || isReadyBundled.value);
});

const dialogRef = useTemplateRef<HTMLDialogElement>('dialogRef');
const searchResultContainerRef = useTemplateRef<HTMLElement>('searchResultContainerRef')
const dialogKey = ref<DialogKey>('none')
const isConnectedToRPC = ref(false);
const isConnecting = ref(false);

// Search functionality
const searchQuery = shallowRef('');
const debouncedSearchQuery = refDebounced(searchQuery, 300)

const searchResultsIsOpen = ref(false);
const isOnSearchResults = ref(false);



onClickOutside(searchResultContainerRef, () => {
    searchResultsIsOpen.value = false;
})


const COPYRIGHT_SYMBOL = '\u00A9';
const TRADEMARK_SYMBOL = '\u2122';
const REGISTERED_SYMBOL = '\u00AE';
const ignoredSymbols = [COPYRIGHT_SYMBOL, TRADEMARK_SYMBOL, REGISTERED_SYMBOL];
const ignoredSymbolsRegex = new RegExp(`[${ignoredSymbols.join('')}]`, 'g');
const fuseOptions = computed<UseFuseOptions<Game>>(() => ({
    fuseOptions: {
        // Prioritize name and aliases for searching, then lastly executables
        keys: [
            { name: 'name', weight: 0.7 },
            { name: 'aliases', weight: 0.2 },
            { name: 'executables.name', weight: 0.1 },
        ],
        getFn: (obj: any, path: string[] | string) => {
            const value = Fuse.config.getFn(obj, path);
            return typeof value === "string"
            ? value.replace(ignoredSymbolsRegex, "")
            : value;
        },
        isCaseSensitive: false,
        threshold: 0.5,        
        // A score of 0indicates a perfect match, while a score of 1 indicates a complete mismatch
        includeScore: true,
        includeMatches: false
    },
    resultLimit: 12,
    matchAllWhenSearchEmpty: false,
}));

const { results: searchResults } = useFuse(debouncedSearchQuery, gameDB, fuseOptions)

// Selected games list
// The games the user added stay until removed: restored from the last session, saved on every change.
const gameList = ref<Game[]>(loadSavedGames());
// Every game currently running (via a dummy process or RPC).
const { rpcGameUid, playingGames, recomputeRunning } = useRunningState(gameList);

// Save when the list itself changes (not on running/stopped), and refresh the saved entries from
// Discord's list whenever a newer one has loaded.
watch(() => serializeGames(gameList.value), () => saveGames(gameList.value));
watch(gameDB, (list) => {
    if (list.length > 0) {
        refreshSavedGames(gameList.value, list);
    }
}, { immediate: true });
const selectedGameId = ref<string | null | undefined>(null);

const selectedGame = computed(() => {
    if (!selectedGameId.value) return null;
    const found = gameList.value.find(g => g.uid === selectedGameId.value);
    return found || null;
});

// The header's Feedback menu includes the selected game in a bug report.
watch(selectedGame, (game) => {
    const steamId = getSteamId(game);
    selectedGameLabel.value = game ? `${game.name}${steamId ? ` (Steam ${steamId})` : ''}` : '';
}, { immediate: true });

function closeSearchResults() {
    searchResultsIsOpen.value = false;
}
function openSearchResults() {
    searchResultsIsOpen.value = true;
}

// Function to add a game to the selected list
function addGameToList(game: Game) {
    if (!gameList.value.some(g => g.id === game.id)) {
        // A clean copy: sharing objects with Discord's list would leak running state into it.
        gameList.value.push({
            uid: randomString(),
            ...cloneGame(game),
        });
    }

    closeSearchResults();
}

const forceRerenderKey = ref(0); 
// Function to remove a game from the selected list
function removeGameFromList(game: Game) {
    const gameId = game.uid;
    gameList.value = gameList.value.filter(game => game.uid !== gameId);
    checked.value = checked.value.filter(uid => uid !== gameId);
    if (selectedGame.value?.uid === gameId) { 
        // selectedGame.value = null;
        selectedGameId.value = null;
        forceRerenderKey.value++; 
    }
}

function selectGame(game: Game) {
    // selectedGame.value = game;
    selectedGameId.value = game?.uid;
    searchResultsIsOpen.value = false;
}



function isExecutableRunning(executable: GameExecutable) {
    // Check if the executable is running
    return executable.is_running ?? false;
}
function isGameExecutableInstalled(executable: GameExecutable) {
    // Check if the executable is installed
    return executable.is_installed ?? false;
}



// Create a dummy game
async function createDummyGame(game: Game | null, executable: GameExecutable) {
    if (!game) {
        return;
    }
    const gameUid = game.uid;
    const gameToInstall = gameList.value.find(g => g.uid === gameUid);
    const executableItem = gameToInstall?.executables.find(exe => exe.name === executable.name);
    if (gameToInstall && executableItem) {
        const payload =  { 
            path: executable.path,
            executable_name: executable.filename,
            app_id: gameToInstall.id,
        }
        const result = await invoke('create_fake_game', payload)
        gameToInstall.is_installed = true;
        executableItem.is_installed = true;
        return true;
    }
}


async function installAndPlay({game, executable}: {game: Game, executable: GameExecutable}) {
    if (!game) {
        return;
    }
    const gameCreated = await createDummyGame(game, executable);
    if (gameCreated) {
        playGame({game, executable});
    } else {
        console.error('Failed to create game');
        addLog('error', 'Failed to create game');
    }
}
// Play game function
async function playGame({game, executable}: {game: Game, executable: GameExecutable}) {
    if (!game) {
        return;
    }
    const gameUid = game.uid;
    try {
        addLog('info', `Playing game: ${game.name}`);
        addLog('info', `Executable: ${executable.name}`);
        // find the game in the list
        const gameToPlay = gameList.value.find(g => g.uid === gameUid);
        const executableItem = gameToPlay?.executables.find(exe => exe.name === executable.name);
        if (gameToPlay && executableItem) {
            const payload =  { 
                name: game.name,
                path: executable.path,
                executable_name: executable.filename,
                app_id: gameToPlay.id,
            } 
            await invoke('run_background_process', payload);
            gameToPlay.is_running = true;
            executableItem.is_running = true; 
        }
        // In a real app, this would invoke a Tauri command to launch the game
       
    } catch (error) {
        console.error('Failed to launch game:', error);
        addLog('error', `Failed to launch ${game.name}: ${error}`);
    }
}

// Stop playing
async function stopPlaying({game, executable}: {game: Game, executable: GameExecutable}) {
    if (!game) {
        return;
    }
    const gameUid = game.uid;
    

    const gameToPlay = gameList.value.find(g => g.uid === gameUid);
    const executableItem = gameToPlay?.executables.find(exe => exe.name === executable.name);
    if (gameToPlay && executableItem) {
        try {
            await invoke('stop_process', {
                exec_name: executable.filename!
            })
            addLog('info', `Stopped game process: ${game.name}`);
            addLog('info', `Stopped Executable: ${executable.name}`);
        } catch (error) {
            console.error('Failed to stop game process:', error);
            const errorMessage = (error instanceof Error) ? error.message : String(error);
            addLog('error', 'Failed to stop game process' + errorMessage);
        } finally {
            // Even if stopping fails, the process is treated as stopped.
            executableItem.is_running = false;
            // The game stays running while any of its other executables is still running.
            recomputeRunning(gameToPlay);
        }
    }
}


// Stop one game completely: its dummy processes, its Steam library entry and its Rich Presence session.
async function stopGame(game: Game) {
    for (const executable of game.executables.filter(exe => exe.is_running)) {
        await stopPlaying({
            game,
            executable: { ...executable, filename: executableFileName(executable.name) },
        });
    }
    // Launched from the Steam library: stop it and remove the fake install again.
    if (game.steam_exe) {
        try {
            const removed = await stopSteamGame(game);
            addLog('info', removed ? `Removed Steam library entry for ${game.name}` : `Stopped ${game.name}; its Steam library entry is kept`);
        } catch (error) {
            addLog('error', `Failed to clean up Steam library entry for ${game.name}: ${error}`);
        }
    }
    if (game.uid && game.uid === rpcGameUid.value) {
        emit('event_disconnect');
        resetRPCState();
    }
}

// Times each running game: remembers when it was last used and stops it when its auto-stop time is up.
const { now, remainingMs } = useGameSessions(gameList, stopGame);
function countdownLabel(game: Game): string {
    const left = remainingMs(game);
    return left === null ? '' : formatCountdown(left);
}

// Stop every running game: dummy processes first, then the Rich Presence connection.
const isStoppingAll = ref(false);
async function stopAll() {
    if (isStoppingAll.value) {
        return;
    }
    isStoppingAll.value = true;
    try {
        for (const game of [...playingGames.value]) {
            await stopGame(game);
        }
        if (isConnectedToRPC.value || isConnecting.value) {
            emit('event_disconnect');
            resetRPCState();
        }
    } finally {
        isStoppingAll.value = false;
    }
}

// Clear every "connected via RPC" flag, e.g. when the connection fails or is dropped.
function resetRPCState() {
    isConnectedToRPC.value = false;
    isConnecting.value = false;
    rpcGameUid.value = null;
    gameList.value.forEach(recomputeRunning);
}

// Emitted by the backend when Discord can't be reached (e.g. Discord isn't running).
listen<{ message: string }>('client_error', (event) => {
    addLog('error', `Discord RPC error: ${event.payload.message}`);
    resetRPCState();
});

async function handleTestRPC(game: Game | null) {
    let state = isConnectedToRPC.value ? 'disconnect' : 'connect';

    if (!game && state === 'connect') {
        showDialog('no_game_selected');
        return;
    }
    if (state === 'disconnect' || isConnecting.value) {
        emit('event_disconnect');
        
        isConnectedToRPC.value = false;
        rpcGameUid.value = null;
        recomputeRunning(game!);
        isConnecting.value = false;
        return;
    }
    showDialog('rpc_message_1');
}

async function continueRPCRisk(game: Game | null) {
    if (!game) {
        return;
    }
    const gameUid = game.uid;
    const gameToTest = gameList.value.find(g => g.uid === gameUid);
    if (gameToTest) {
        isConnecting.value = true;
        invoke('connect_to_discord_rpc_3', {
            activity_json: JSON.stringify({
                app_id: gameToTest.id,
            }),
        })
        .then(() => {
            isConnectedToRPC.value = true;
            rpcGameUid.value = gameToTest.uid ?? null;
            recomputeRunning(gameToTest);
            isConnecting.value = false;
        })
        .catch((error) => {
            addLog('error', `RPC failed: ${error}`);
            resetRPCState();
        })

        hideDialog();
    }
}

// ---- Sidebar: filter, favorites and order ----
const gameFilter = ref('');
const visibleGames = computed(() => {
    const query = gameFilter.value.trim().toLowerCase();
    const matches = (game: Game) => !query
        || game.name.toLowerCase().includes(query)
        || (game.aliases ?? []).some(alias => alias.toLowerCase().includes(query));
    const order = settings.sortOrder;
    return gameList.value
        .map((game, index) => ({ game, index }))
        .filter(({ game }) => matches(game))
        .sort((a, b) => {
            if (!!a.game.favorite !== !!b.game.favorite) return a.game.favorite ? -1 : 1;
            if (order === 'name') return a.game.name.localeCompare(b.game.name) || a.index - b.index;
            if (order === 'recent') return (b.game.last_used ?? 0) - (a.game.last_used ?? 0) || a.index - b.index;
            return a.index - b.index;
        })
        .map(({ game }) => game);
});
function toggleFavorite(game: Game) {
    game.favorite = !game.favorite;
}

// ---- Run several games at once ----
const selectMode = ref(false);
const checked = ref<string[]>([]);
const isBatchRunning = ref(false);
const batchMessage = ref('');

const isChecked = (game: Game) => !!game.uid && checked.value.includes(game.uid);
function toggleSelectMode() {
    selectMode.value = !selectMode.value;
    checked.value = [];
    batchMessage.value = '';
}
function toggleChecked(game: Game) {
    const uid = game.uid;
    if (!uid) return;
    checked.value = checked.value.includes(uid) ? checked.value.filter(id => id !== uid) : [...checked.value, uid];
}
function checkAllVisible() {
    checked.value = visibleGames.value.map(game => game.uid).filter((uid): uid is string => !!uid);
}

// Is a dummy exe with this file name already running for some game? Two of them would share a process name.
function exeInUse(file: string) {
    return gameList.value.some(game =>
        game.steam_exe?.toLowerCase() === file
        || game.executables.some(exe => exe.is_running && executableFileName(exe.name).toLowerCase() === file));
}

let steamBatchConfirmed = false;
async function confirmSteamBatch(): Promise<boolean> {
    if (steamBatchConfirmed) {
        return true;
    }
    steamBatchConfirmed = await ask(
        'Some of the selected games have no real executable listed, so they will be added to your Steam library '
        + 'as fake installs (a manifest file and a folder with a dummy exe).\n\n'
        + (settings.keepSteamEntries
            ? 'They stay there after the game stops until you remove them in Settings. '
            : 'They are removed again when the game stops. ')
        + 'Games Steam already has installed are never touched.',
        { title: 'Add to Steam library?', kind: 'warning', okLabel: 'Continue', cancelLabel: 'Cancel' },
    );
    return steamBatchConfirmed;
}

// Start one game the way its own page would. Returns why it was skipped, or '' when it started.
async function startForBatch(game: Game): Promise<string> {
    const candidates = game.executables.filter(exe => exe.os === getCurrentOS() && isValidExecutablePath(exe.name));
    const real = candidates.filter(exe => !exe.is_auto_generated);
    const steamId = getSteamId(game);

    // No real executable is listed: newer quests look for a Steam install, so use the Steam library if there is one.
    if (real.length === 0 && steamId) {
        const info = await fetchSteamInfo(steamId);
        if (info.steam_found) {
            if (!(await confirmSteamBatch())) {
                return 'Steam library change declined';
            }
            const guess = guessExeName(info.installdir, info.exe, game.name);
            const launched = await launchSteamGame(game, steamId, info.installdir ?? defaultInstallFolder(game.name), guess.exe);
            addLog('info', `Launched ${game.name} from the Steam library: ${launched.exe_path}`);
            return '';
        }
    }

    const pick = real.find(exe => !exe.is_launcher) ?? real[0] ?? candidates[0];
    if (!pick) {
        return 'no executable for Windows';
    }
    const file = executableFileName(pick.name).toLowerCase();
    if (exeInUse(file)) {
        return `${file} is already used by a running game`;
    }
    const executable: GameExecutable = {
        ...pick,
        path: executableDirectory(pick.name, path.sep()),
        filename: executableFileName(pick.name),
    };
    if (!pick.is_installed && !(await createDummyGame(game, executable))) {
        return 'could not create the dummy exe';
    }
    await playGame({ game, executable });
    return pick.is_running ? '' : 'could not start, see Diagnostics';
}

async function runSelected() {
    const games = gameList.value.filter(isChecked);
    if (games.length === 0 || isBatchRunning.value) {
        return;
    }
    isBatchRunning.value = true;
    batchMessage.value = '';
    const started: string[] = [];
    const skipped: string[] = [];
    try {
        for (const game of games) {
            if (game.is_running) {
                skipped.push(`${game.name} (already running)`);
                continue;
            }
            try {
                const reason = await startForBatch(game);
                if (reason) {
                    skipped.push(`${game.name} (${reason})`);
                } else {
                    started.push(game.name);
                }
            } catch (error) {
                skipped.push(`${game.name} (${error})`);
                addLog('error', `Failed to start ${game.name}: ${error}`);
            }
        }
    } finally {
        isBatchRunning.value = false;
    }
    batchMessage.value = `Started ${started.length} of ${games.length}.` + (skipped.length ? ` Skipped: ${skipped.join('; ')}.` : '');
    addLog(skipped.length ? 'warning' : 'info', `Run selected: ${batchMessage.value}`);
    selectMode.value = false;
    checked.value = [];
}

function handleSearchBlur() {
    setTimeout(() => {
        if (!isOnSearchResults.value) {
            searchResultsIsOpen.value = false;
        }
    }, 200);
}

function showDialog(key: DialogKey) {
    dialogKey.value = key;
    dialogRef.value?.showModal();
}

function hideDialog() {
    dialogRef.value?.close();
}

// ---- Keyboard shortcuts (only on this page) ----
const searchInputRef = useTemplateRef<HTMLInputElement>('searchInputRef');
const filterInputRef = useTemplateRef<HTMLInputElement>('filterInputRef');

useEventListener('keydown', (event: KeyboardEvent) => {
    if (page.value !== Pages.HOME) {
        return;
    }
    const ctrl = event.ctrlKey || event.metaKey;
    if (ctrl && event.key.toLowerCase() === 'f') {
        // The filter when the list is long enough to have one, otherwise the search box.
        event.preventDefault();
        (filterInputRef.value ?? searchInputRef.value)?.focus();
        (filterInputRef.value ?? searchInputRef.value)?.select();
    } else if (ctrl && event.key === 'Enter') {
        if (selectMode.value && checked.value.length > 0) {
            event.preventDefault();
            runSelected();
        }
    } else if (event.key === 'Escape') {
        if (document.activeElement === filterInputRef.value && gameFilter.value) {
            gameFilter.value = '';
        } else if (selectMode.value) {
            toggleSelectMode();
        } else if (document.activeElement instanceof HTMLInputElement) {
            document.activeElement.blur();
        }
    }
});

provide<GameActionsProvider>(GameActionsKey, {
    isExecutableRunning,
    isGameExecutableInstalled,
});
</script>

<template>
    <div class="flex h-full min-h-0">
        <!-- Confirmation dialog -->
        <dialog id="dialog" class="dialogStyle m-auto w-[min(92vw,28rem)] rounded-2xl border border-zinc-200 bg-white p-0 text-zinc-700 shadow-2xl dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-300"
            ref="dialogRef">
            <div class="flex flex-col gap-5 p-6">
                <div v-if="dialogKey === 'rpc_message_1'" class="space-y-3 text-sm leading-relaxed">
                    <div class="flex items-center gap-2 text-amber-500">
                        <svg class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495ZM10 6a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 10 6Zm0 9a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z" clip-rule="evenodd"/></svg>
                        <span class="font-semibold text-zinc-900 dark:text-white">Experimental feature</span>
                    </div>
                    <p>
                        This is still in development. It works by sending an RPC with the real game ID, instead of letting
                        Discord detect a running application.
                    </p>
                    <p>This may flag your account as suspicious for self-botting.</p>
                </div>

                <p v-if="dialogKey === 'no_game_selected'" class="text-sm">
                    No game selected. Please select a game from the sidebar.
                </p>

                <div class="flex justify-end gap-2">
                    <button class="btn-ghost" @click="hideDialog()">
                        <span v-if="dialogKey == 'rpc_message_1'">Cancel</span>
                        <span v-else>OK</span>
                    </button>
                    <button v-if="dialogKey === 'rpc_message_1'" class="btn-primary" @click="continueRPCRisk(selectedGame)">
                        Accept risk and continue
                    </button>
                </div>
            </div>
        </dialog>

        <!-- Sidebar -->
        <aside class="flex w-72 shrink-0 flex-col border-r border-zinc-200 bg-white dark:border-zinc-800 dark:bg-zinc-900">
            <!-- Search -->
            <div class="relative border-b border-zinc-200 p-3 dark:border-zinc-800" ref="searchResultContainerRef">
                <div class="relative">
                    <svg class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-zinc-400" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M9 3.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11ZM2 9a7 7 0 1 1 12.452 4.391l3.328 3.329a.75.75 0 1 1-1.06 1.06l-3.329-3.328A7 7 0 0 1 2 9Z" clip-rule="evenodd"/></svg>
                    <input ref="searchInputRef" v-model="searchQuery" type="text" placeholder="Search games to add…"
                        class="h-9 w-full rounded-lg border border-zinc-200 bg-zinc-50 pl-9 pr-3 text-sm text-zinc-900 outline-none transition placeholder:text-zinc-400 focus:border-indigo-500 focus:bg-white focus:ring-4 focus:ring-indigo-500/15 dark:border-zinc-800 dark:bg-zinc-950 dark:text-white dark:focus:bg-zinc-950"
                        @focus="openSearchResults" @blur="handleSearchBlur" />
                </div>

                <div v-if="searchResultsIsOpen" @click="isOnSearchResults = true"
                    class="absolute inset-x-3 top-full z-50 mt-1 max-h-96 overflow-y-auto rounded-xl border border-zinc-200 bg-white p-1.5 shadow-xl dark:border-zinc-700 dark:bg-zinc-900">
                    <template v-if="searchResults.length > 0">
                        <div v-for="game in searchResults" :key="game.item.id"
                            class="flex items-center gap-2.5 rounded-lg p-2 transition hover:bg-zinc-50 dark:hover:bg-zinc-800/60">
                            <div class="avatar h-8 w-8 shrink-0 text-xs">{{ game.item.name.slice(0, 2).toUpperCase() }}</div>
                            <div class="min-w-0 flex-1">
                                <div class="truncate text-sm font-medium text-zinc-900 dark:text-white">{{ game.item.name }}</div>
                                <div class="text-[11px]"
                                    :class="game.item.executables.length ? 'text-zinc-400' : 'text-amber-500'">
                                    {{ game.item.executables.length
                                        ? game.item.executables.length + ' executable' + (game.item.executables.length > 1 ? 's' : '')
                                        : 'no executables registered' }}
                                </div>
                            </div>
                            <button @click="addGameToList(game.item)" class="btn-primary shrink-0 !px-2.5 !py-1 text-xs">Add</button>
                        </div>
                    </template>
                    <div v-else class="p-3 text-xs text-zinc-500 dark:text-zinc-400">
                        Search for a game by name, then click <span class="font-medium">Add</span> to keep it in the sidebar.
                    </div>
                </div>
            </div>

            <!-- Added games -->
            <div class="flex items-center justify-between px-4 pb-1 pt-3">
                <div class="flex items-center gap-2">
                    <span class="text-[11px] font-semibold uppercase tracking-wider text-zinc-400">Games</span>
                    <span class="chip">{{ gameList.length }}</span>
                </div>
                <button v-if="gameList.length > 0" class="rounded-md px-2 py-0.5 text-[11px] font-medium transition"
                    :class="selectMode
                        ? 'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400'
                        : 'text-zinc-500 hover:bg-zinc-100 dark:text-zinc-400 dark:hover:bg-zinc-800'"
                    :title="selectMode ? 'Leave select mode (Esc)' : 'Select several games to run together'"
                    :aria-pressed="selectMode" @click="toggleSelectMode()">
                    {{ selectMode ? 'Done' : 'Select' }}
                </button>
            </div>

            <!-- Pick several games and start them all -->
            <div v-if="selectMode" class="space-y-2 border-b border-zinc-200 px-3 pb-3 pt-1 dark:border-zinc-800">
                <div class="flex items-center justify-between text-xs">
                    <span class="text-zinc-500 dark:text-zinc-400">{{ checked.length }} selected</span>
                    <span class="flex gap-3">
                        <button class="text-indigo-600 hover:underline dark:text-indigo-400" @click="checkAllVisible()">All</button>
                        <button class="text-indigo-600 hover:underline disabled:opacity-40 dark:text-indigo-400" :disabled="checked.length === 0" @click="checked = []">None</button>
                    </span>
                </div>
                <button class="btn-primary w-full !py-1.5 text-xs" :disabled="checked.length === 0 || isBatchRunning" title="Run the selected games (Ctrl+Enter)" @click="runSelected()">
                    {{ isBatchRunning ? 'Starting…' : `Run selected (${checked.length})` }}
                </button>
            </div>
            <p v-if="batchMessage" class="mx-3 mt-2 break-words rounded-lg bg-zinc-100 px-3 py-2 text-[11px] text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300">
                {{ batchMessage }}
            </p>

            <!-- Filter the added games once the list gets long -->
            <div v-if="gameList.length >= 5" class="px-3 pb-1 pt-1">
                <input ref="filterInputRef" v-model="gameFilter" type="text" placeholder="Filter your games…  (Ctrl+F)" aria-label="Filter your games"
                    class="h-8 w-full rounded-lg border border-zinc-200 bg-zinc-50 px-3 text-xs text-zinc-900 outline-none transition placeholder:text-zinc-400 focus:border-indigo-500 focus:ring-4 focus:ring-indigo-500/15 dark:border-zinc-800 dark:bg-zinc-950 dark:text-white" />
            </div>

            <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
                <div v-if="gameList.length === 0"
                    class="mx-1 mt-2 rounded-xl border border-dashed border-zinc-300 px-4 py-8 text-center text-xs text-zinc-500 dark:border-zinc-700 dark:text-zinc-400">
                    No games yet. Use the search above to add one.
                </div>

                <p v-else-if="visibleGames.length === 0" class="px-3 py-6 text-center text-xs text-zinc-500 dark:text-zinc-400">
                    No games match "{{ gameFilter }}".
                </p>

                <ul v-else class="space-y-0.5">
                    <li v-for="game in visibleGames" :key="game.id"
                        class="group flex cursor-pointer items-center gap-2.5 rounded-lg px-2 py-2 transition"
                        :class="(selectMode ? isChecked(game) : selectedGame?.uid === game.uid)
                            ? 'bg-indigo-500/10 text-zinc-900 dark:text-white'
                            : 'hover:bg-zinc-100 dark:hover:bg-zinc-800/60'"
                        @click="selectMode ? toggleChecked(game) : selectGame(game)">
                        <input v-if="selectMode" type="checkbox" :checked="isChecked(game)" tabindex="-1" aria-hidden="true"
                            class="pointer-events-none h-4 w-4 shrink-0 accent-indigo-600" />
                        <div class="avatar h-8 w-8 shrink-0 text-xs">{{ game.name.slice(0, 2).toUpperCase() }}</div>
                        <div class="min-w-0 flex-1">
                            <div class="truncate text-sm font-medium text-zinc-800 dark:text-zinc-100">{{ game.name }}</div>
                            <div v-if="game.is_running" class="flex items-center gap-1.5 text-[11px] font-medium text-emerald-500">
                                <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-500"></span> Running
                                <span v-if="countdownLabel(game)" class="font-mono font-normal text-zinc-400">· {{ countdownLabel(game) }}</span>
                            </div>
                            <div v-else-if="game.last_used" class="text-[11px] text-zinc-400">Last used {{ formatAgo(game.last_used, now) }}</div>
                        </div>
                        <button v-if="!selectMode" @click.stop="toggleFavorite(game)"
                            :title="game.favorite ? 'Remove from favorites' : 'Add to favorites'"
                            :aria-label="game.favorite ? 'Remove from favorites' : 'Add to favorites'" :aria-pressed="!!game.favorite"
                            class="rounded-md p-1 transition hover:bg-amber-500/10"
                            :class="game.favorite ? 'text-amber-500' : 'text-zinc-400 opacity-0 hover:text-amber-500 group-hover:opacity-100'">
                            <svg class="h-4 w-4" viewBox="0 0 20 20" :fill="game.favorite ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"><path d="M10 2.5l2.2 4.6 5 .7-3.6 3.5.9 5-4.5-2.4-4.5 2.4.9-5L2.8 7.8l5-.7L10 2.5Z"/></svg>
                        </button>
                        <button v-if="!game.is_running && !selectMode" @click.stop="removeGameFromList(game)" title="Remove"
                            class="rounded-md p-1 text-zinc-400 opacity-0 transition hover:bg-red-500/10 hover:text-red-500 group-hover:opacity-100">
                            <svg class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path d="M6.28 5.22a.75.75 0 0 0-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 1 0 1.06 1.06L10 11.06l3.72 3.72a.75.75 0 1 0 1.06-1.06L11.06 10l3.72-3.72a.75.75 0 0 0-1.06-1.06L10 8.94 6.28 5.22Z"/></svg>
                        </button>
                    </li>
                </ul>
            </div>

            <!-- Status: green while at least one game is running, orange when idle -->
            <div class="border-t border-zinc-200 p-3 dark:border-zinc-800">
                <div class="rounded-xl bg-zinc-50 p-3 dark:bg-zinc-950/60">
                    <div class="flex items-center gap-2 text-xs">
                        <span class="h-2 w-2 shrink-0 rounded-full"
                            :class="playingGames.length > 0 ? 'animate-pulse bg-emerald-500' : 'bg-amber-500'"></span>
                        <span class="font-medium"
                            :class="playingGames.length > 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-amber-600 dark:text-amber-400'">
                            {{ playingGames.length > 0 ? `Playing (${playingGames.length})` : 'Idle' }}
                        </span>
                    </div>
                    <ul v-if="playingGames.length > 0" class="mt-2 max-h-24 space-y-1 overflow-y-auto text-xs text-zinc-600 dark:text-zinc-300">
                        <li v-for="game in playingGames" :key="game.id" class="flex items-center justify-between gap-2" :title="game.name">
                            <span class="truncate">{{ game.name }}</span>
                            <span v-if="countdownLabel(game)" class="shrink-0 font-mono text-[11px] text-zinc-400" title="Time until auto-stop">{{ countdownLabel(game) }}</span>
                        </li>
                    </ul>
                    <p v-else class="mt-1 text-xs text-zinc-500 dark:text-zinc-400">Not playing any game</p>
                    <button class="mt-3 w-full rounded-lg px-3 py-1.5 text-xs font-medium transition"
                        :class="playingGames.length > 0
                            ? 'bg-red-500/10 text-red-500 hover:bg-red-500/20'
                            : 'bg-zinc-200/60 text-zinc-400 dark:bg-zinc-800/60 dark:text-zinc-500'"
                        :disabled="playingGames.length === 0 || isStoppingAll"
                        @click="stopAll()">
                        {{ playingGames.length > 1 ? 'Stop all' : 'Stop' }}
                    </button>
                </div>
            </div>
        </aside>

        <!-- Game list fetch status -->
        <Transition
            enter-active-class="transition duration-300 ease-out"
            leave-active-class="transition duration-500 ease-in"
            enter-from-class="opacity-0 translate-y-2"
            enter-to-class="opacity-100 translate-y-0"
            leave-to-class="opacity-0"
        >
            <div class="fixed bottom-4 right-4 z-20 space-y-1 rounded-xl border border-zinc-200 bg-white/90 px-3 py-2 text-xs text-zinc-500 shadow-lg backdrop-blur dark:border-zinc-800 dark:bg-zinc-900/90 dark:text-zinc-400"
                v-if="shouldShowNotificationContainer && !allFetchDone">
                <div v-if="isLoadingGH" class="flex items-center gap-2">
                    <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-500"></span>
                    Fetching game list from GitHub mirror…
                </div>
                <TimedNotification :is-ready="isReadyGH" :duration="1500">
                    Game list from mirror fetched <span class="text-emerald-500">✓</span>
                </TimedNotification>

                <div v-if="isLoadingDiscord" class="flex items-center gap-2">
                    <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-500"></span>
                    Fetching game list directly from Discord…
                </div>
                <TimedNotification :is-ready="isReadyDiscord" :duration="1500">
                    Game list from Discord fetched <span class="text-emerald-500">✓</span>
                </TimedNotification>

                <div v-if="isLoadingBundled" class="flex items-center gap-2">
                    <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-500"></span>
                    Loading bundled game list…
                </div>
                <TimedNotification :is-ready="isReadyBundled" :duration="1500">
                    Bundled game list pre-loaded <span class="text-emerald-500">✓</span>
                </TimedNotification>
            </div>
        </Transition>

        <!-- Game actions -->
        <section class="min-w-0 flex-1 overflow-y-auto" :key="forceRerenderKey">
            <div v-if="!selectedGame" class="flex h-full flex-col items-center justify-center gap-3 px-6 text-center">
                <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-zinc-100 text-zinc-400 dark:bg-zinc-900">
                    <svg class="h-7 w-7" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M15.75 15.75 12 12m0 0L8.25 8.25M12 12l3.75-3.75M12 12l-3.75 3.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"/></svg>
                </div>
                <div>
                    <div class="font-medium text-zinc-900 dark:text-white">No game selected</div>
                    <p class="mt-1 text-sm text-zinc-500 dark:text-zinc-400">Pick a game from the sidebar to see its actions.</p>
                </div>
            </div>

            <div v-else class="mx-auto max-w-2xl space-y-5 p-6">
                <div class="flex items-center gap-4">
                    <div class="avatar h-14 w-14 text-lg">{{ selectedGame.name.slice(0, 2).toUpperCase() }}</div>
                    <div class="min-w-0">
                        <div class="flex items-center gap-1.5">
                            <h2 class="truncate text-xl font-semibold text-zinc-900 dark:text-white">{{ selectedGame.name }}</h2>
                            <span class="relative inline-flex shrink-0 items-center">
                                <span class="absolute left-1/2 top-1/2 h-1.5 w-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white"></span>
                                <IconVerified class="relative h-5 w-5 text-indigo-500 dark:text-indigo-400" />
                            </span>
                        </div>
                        <div class="text-xs text-zinc-400">
                            <span class="font-mono">{{ selectedGame.id }}</span>
                            <span v-if="selectedGame.last_used && !selectedGame.is_running"> · Last used {{ formatAgo(selectedGame.last_used, now) }}</span>
                        </div>
                    </div>
                </div>

                <div v-if="selectedGame.aliases && selectedGame.aliases.length > 0" class="flex flex-wrap gap-1">
                    <span v-for="alias in selectedGame.aliases" :key="alias" class="chip">{{ alias }}</span>
                </div>

                <div class="card">
                    <GameExecutables :game="selectedGame"
                        @play="playGame"
                        @install_and_play="installAndPlay" />
                </div>
                <SteamLaunch :game="selectedGame" />
                <AutoStopCard :game="selectedGame" :remaining-ms="remainingMs(selectedGame)" />


                <div class="card">
                    <h3 class="mb-1 text-sm font-medium text-zinc-900 dark:text-white">Rich Presence</h3>
                    <p class="mb-3 text-xs text-zinc-500 dark:text-zinc-400">
                        Experimental. Sends an RPC using this game's ID instead of running a process.
                    </p>
                    <button @click="handleTestRPC(selectedGame)" class="w-full"
                        :class="isConnecting || isConnectedToRPC ? 'btn-danger' : 'btn-secondary'">
                        {{ isConnecting || isConnectedToRPC ? 'Disconnect from Discord Gateway' : 'Test RPC' }}
                    </button>
                </div>
            </div>
        </section>
    </div>
</template>

<style scoped>
@reference "../theme/style.css";

.dialogStyle::backdrop {
    @apply bg-black/60 backdrop-blur-sm;
}
</style>
