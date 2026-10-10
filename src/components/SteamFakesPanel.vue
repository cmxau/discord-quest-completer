<script setup lang="ts">
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';
import { useIntervalFn } from '@vueuse/core';
import { useGlobalState } from '@/composables/app-state';
import { useSettings } from '@/composables/settings';

interface SteamFake {
    steam_id: string;
    folder_name: string;
    exe_name: string;
    exe_path: string;
    acf_path: string;
}

const { addLog } = useGlobalState();
const { settings } = useSettings();

const fakes = ref<SteamFake[]>([]);
const alive = ref<Set<string>>(new Set());
const busy = ref(false);
const error = ref('');

async function refresh() {
    try {
        fakes.value = await invoke<SteamFake[]>('list_steam_fakes');
        alive.value = new Set(await invoke<string[]>('running_exes'));
    } catch (e) {
        error.value = String(e);
    }
}
useIntervalFn(refresh, 3000, { immediateCallback: true });

const isRunning = (fake: SteamFake) => alive.value.has(fake.exe_name.toLowerCase());
const runningCount = computed(() => fakes.value.filter(isRunning).length);

async function remove(fake: SteamFake | null) {
    if (busy.value) {
        return;
    }
    if (!fake) {
        const running = runningCount.value ? ` ${runningCount.value} of them ${runningCount.value === 1 ? 'is' : 'are'} running and will be stopped.` : '';
        const ok = await ask(
            `Remove all ${fakes.value.length} Steam entries the app created?${running}\n\nOnly files this app made are deleted. Games Steam installed are never touched.`,
            { title: 'Remove all Steam entries', kind: 'warning', okLabel: 'Remove all', cancelLabel: 'Cancel' },
        );
        if (!ok) {
            return;
        }
    }
    busy.value = true;
    error.value = '';
    try {
        const removed = await invoke<number>('remove_steam_fakes', { steam_id: fake?.steam_id ?? null });
        addLog('info', fake ? `Removed Steam entry for ${fake.folder_name} (${fake.steam_id})` : `Removed ${removed} Steam entries`);
    } catch (e) {
        error.value = String(e);
        addLog('error', `Removing Steam entries failed: ${e}`);
    } finally {
        busy.value = false;
        await refresh();
    }
}
</script>

<template>
    <div>
        <div class="flex items-center justify-between gap-3">
            <p class="text-xs text-zinc-500 dark:text-zinc-400">
                Entries the app added to your Steam library for games launched from there.
                {{ settings.keepSteamEntries ? 'They are kept after a game stops, until you remove them here.' : 'Normally removed when you stop the game.' }}
            </p>
            <button v-if="fakes.length > 1" class="btn-danger shrink-0 !px-3 !py-1.5 text-xs" :disabled="busy" @click="remove(null)">
                Remove all
            </button>
        </div>

        <p v-if="fakes.length === 0"
            class="mt-3 rounded-lg border border-dashed border-zinc-300 px-3 py-4 text-center text-xs text-zinc-500 dark:border-zinc-700 dark:text-zinc-400">
            No Steam entries right now.
        </p>

        <ul v-else class="mt-3 space-y-2">
            <li v-for="fake in fakes" :key="fake.steam_id"
                class="flex items-center gap-3 rounded-xl border border-zinc-200 p-3 dark:border-zinc-800">
                <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                        <span class="truncate text-sm font-medium text-zinc-900 dark:text-white">{{ fake.folder_name || fake.exe_name }}</span>
                        <span class="chip shrink-0">App {{ fake.steam_id }}</span>
                        <span v-if="isRunning(fake)" class="chip shrink-0 !bg-emerald-500/15 !text-emerald-600 dark:!text-emerald-400">running</span>
                        <span v-else class="chip shrink-0">kept</span>
                    </div>
                    <div class="mt-0.5 truncate font-mono text-[11px] text-zinc-400" :title="fake.exe_path">{{ fake.exe_path }}</div>
                </div>
                <button class="btn-ghost shrink-0 !px-3 !py-1.5 text-xs" :disabled="busy" @click="remove(fake)">
                    {{ isRunning(fake) ? 'Stop and remove' : 'Remove' }}
                </button>
            </li>
        </ul>

        <p v-if="error" class="mt-3 break-words rounded-lg bg-red-500/10 px-3 py-2 text-xs text-red-500">{{ error }}</p>
    </div>
</template>
