<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { useGlobalState } from '@/composables/app-state';
import { fetchSteamInfo, getSteamId, guessExeName, launchSteamGame, type SteamInfo } from '@/composables/steam-launch';
import type { Game } from '@/types/types';

const props = defineProps<{ game: Game }>();

const { addLog } = useGlobalState();

const steamId = computed(() => getSteamId(props.game));
const info = ref<SteamInfo | null>(null);
const loading = ref(false);
const installDir = ref('');
const exeName = ref('');
const exeGuessed = ref(false);
const error = ref('');
const launching = ref(false);

const canLaunch = computed(() =>
    !!info.value?.steam_found && !!installDir.value.trim() && !!exeName.value.trim()
    && !props.game.steam_exe && !launching.value
);

watch(() => props.game.uid, async () => {
    info.value = null;
    error.value = '';
    installDir.value = '';
    exeName.value = '';
    if (!steamId.value) {
        return;
    }
    loading.value = true;
    try {
        info.value = await fetchSteamInfo(steamId.value);
        installDir.value = info.value.installdir ?? props.game.name;
        const guess = guessExeName(info.value.installdir, info.value.exe);
        exeName.value = guess.exe;
        exeGuessed.value = guess.guessed;
    } catch (e) {
        error.value = String(e);
    } finally {
        loading.value = false;
    }
}, { immediate: true });

// Ask once per session before touching the Steam library.
let acknowledged = false;
async function confirmSteamChange(): Promise<boolean> {
    if (acknowledged) {
        return true;
    }
    const ok = await ask(
        'This adds a temporary fake install of this game to your Steam library '
        + '(a manifest file and a folder with a dummy exe).\n\n'
        + 'It is removed again when you press Stop. Games Steam already has installed are never touched.\n\n'
        + 'Steam may show the game as installed while it runs. Do not launch it from Steam.',
        { title: 'Add to Steam library?', kind: 'warning', okLabel: 'Continue', cancelLabel: 'Cancel' },
    );
    acknowledged = ok;
    return ok;
}

async function launch() {
    if (!steamId.value || !canLaunch.value) {
        return;
    }
    error.value = '';
    launching.value = true;
    try {
        if (!(await confirmSteamChange())) {
            return;
        }
        await launchSteamGame(props.game, steamId.value, installDir.value.trim(), exeName.value.trim());
        addLog('info', `Launched ${props.game.name} from the Steam library (${exeName.value.trim()})`);
    } catch (e) {
        error.value = String(e);
        addLog('error', `Steam launch failed: ${e}`);
    } finally {
        launching.value = false;
    }
}
</script>

<template>
    <div v-if="steamId" class="card">
        <div class="mb-1 flex items-center justify-between">
            <h3 class="text-sm font-medium text-zinc-900 dark:text-white">Steam library</h3>
            <span class="chip">App {{ steamId }}</span>
        </div>
        <p class="mb-3 text-xs text-zinc-500 dark:text-zinc-400">
            Newer quests check for a Steam install. This puts the dummy exe where Steam keeps the game, and removes it again when you press Stop.
        </p>

        <p v-if="loading" class="text-xs text-zinc-500 dark:text-zinc-400">Looking up the install folder…</p>

        <p v-else-if="info && !info.steam_found"
            class="rounded-lg bg-amber-500/10 px-3 py-2 text-xs text-amber-600 dark:text-amber-400">
            Steam isn't installed on this PC, so this can't be used.
        </p>

        <template v-else-if="info">
            <label class="mb-1 block text-xs font-medium text-zinc-600 dark:text-zinc-300">Install folder</label>
            <input v-model="installDir" type="text" spellcheck="false"
                class="mb-3 h-9 w-full rounded-lg border border-zinc-200 bg-zinc-50 px-3 font-mono text-xs text-zinc-900 outline-none transition focus:border-indigo-500 focus:ring-4 focus:ring-indigo-500/15 dark:border-zinc-800 dark:bg-zinc-950 dark:text-white" />

            <label class="mb-1 flex items-center gap-2 text-xs font-medium text-zinc-600 dark:text-zinc-300">
                Executable
                <span v-if="exeGuessed" class="chip !bg-amber-500/15 !text-amber-600 dark:!text-amber-400">unverified guess</span>
            </label>
            <input v-model="exeName" type="text" spellcheck="false" placeholder="game.exe"
                class="mb-1 h-9 w-full rounded-lg border border-zinc-200 bg-zinc-50 px-3 font-mono text-xs text-zinc-900 outline-none transition focus:border-indigo-500 focus:ring-4 focus:ring-indigo-500/15 dark:border-zinc-800 dark:bg-zinc-950 dark:text-white" />
            <p v-if="exeGuessed" class="mb-3 text-[11px] text-zinc-500 dark:text-zinc-400">
                Steam launches this game through a launcher, so the real exe name isn't known. Edit it if the quest isn't detected.
            </p>
            <div v-else class="mb-3"></div>

            <button class="w-full" :class="game.steam_exe ? 'btn-ghost' : 'btn-primary'"
                :disabled="!canLaunch" @click="launch()">
                {{ game.steam_exe ? 'Running from Steam library' : (launching ? 'Starting…' : 'Launch in Steam library') }}
            </button>
        </template>

        <p v-if="error" class="mt-3 break-words rounded-lg bg-red-500/10 px-3 py-2 text-xs text-red-500">{{ error }}</p>
    </div>
</template>
