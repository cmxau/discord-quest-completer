<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { MAX_MINUTES, useSettings, useTheme, type SortOrder } from '@/composables/settings';
import { useGlobalState } from '@/composables/app-state';
import { useDiscordStatus } from '@/composables/discord-status';
import { useUpdates } from '@/composables/updates';
import SettingRow from '@/components/SettingRow.vue';
import FeedbackSection from '@/components/FeedbackSection.vue';
import HowToUse from '@/components/HowToUse.vue';
import SteamFakesPanel from '@/components/SteamFakesPanel.vue';
import ToggleSwitch from '@/components/ToggleSwitch.vue';

const { settings } = useSettings();
const { store: themeChoice } = useTheme();
const { isGameListLoading, requestGameListRefresh } = useGlobalState();
const discord = useDiscordStatus();
const updates = useUpdates();

// ---- Appearance ----
const themes = [
    { value: 'auto', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
] as const;

const version = ref('');
onMounted(async () => {
    try {
        version.value = await getVersion();
    } catch {
        version.value = '';
    }
});

// ---- Start with Windows (kept in the Windows registry, so its state is read from there) ----
const startWithWindows = ref(false);
const autostartBusy = ref(false);
const autostartError = ref('');

async function readAutostart() {
    try {
        startWithWindows.value = await invoke<boolean>('get_autostart');
    } catch {
        startWithWindows.value = false;
    }
}
onMounted(readAutostart);

async function applyAutostart(enabled: boolean) {
    autostartBusy.value = true;
    autostartError.value = '';
    try {
        await invoke('set_autostart', { enabled, minimized: settings.startMinimized });
    } catch (e) {
        autostartError.value = String(e);
    } finally {
        autostartBusy.value = false;
        await readAutostart(); // show what Windows really has
    }
}

function setStartMinimized(value: boolean) {
    settings.startMinimized = value;
    if (startWithWindows.value) {
        applyAutostart(true); // rewrite the startup command with or without --minimized
    }
}

// ---- Auto-stop ----
const customCount = computed(() => Object.keys(settings.gameMinutes).length);
const defaultMinutes = computed({
    get: () => settings.autoStopMinutes,
    set: (value: number | string) => {
        const minutes = Math.round(Number(value));
        if (Number.isFinite(minutes) && minutes >= 1) {
            settings.autoStopMinutes = Math.min(MAX_MINUTES, minutes);
        }
    },
});

// ---- Game list ----
const orders: { value: SortOrder; label: string }[] = [
    { value: 'added', label: 'Order added' },
    { value: 'name', label: 'Name' },
    { value: 'recent', label: 'Recently used' },
];
</script>

<template>
    <div class="mx-auto max-w-2xl space-y-4 p-6">
        <h1 class="text-lg font-semibold text-zinc-900 dark:text-white">Settings</h1>

        <!-- Closed until opened -->
        <details class="card group">
            <summary class="flex cursor-pointer list-none items-center justify-between gap-3 [&::-webkit-details-marker]:hidden">
                <span class="card-title">How to use</span>
                <svg class="h-4 w-4 shrink-0 text-zinc-400 transition-transform group-open:rotate-180" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m5 8 5 5 5-5"/></svg>
            </summary>
            <div class="mt-3">
                <HowToUse />
            </div>
        </details>

        <!-- 1. What you do most: running games -->
        <section class="card divide-y divide-zinc-100 dark:divide-zinc-800">
            <h2 class="card-title pb-3">Running games</h2>
            <SettingRow title="Stop games automatically" description="So a dummy game is never left running by mistake.">
                <ToggleSwitch label="Stop games automatically" v-model="settings.autoStopEnabled" />
            </SettingRow>
            <SettingRow title="Default time" description="Most quests need 15 minutes of play. The rest is a margin.">
                <div class="flex items-center gap-2">
                    <input v-model="defaultMinutes" type="number" min="1" :max="MAX_MINUTES" :disabled="!settings.autoStopEnabled"
                        aria-label="Default auto-stop minutes"
                        class="h-9 w-20 rounded-lg border border-zinc-200 bg-zinc-50 px-3 text-sm text-zinc-900 outline-none transition focus:border-indigo-500 focus:ring-4 focus:ring-indigo-500/15 disabled:opacity-50 dark:border-zinc-800 dark:bg-zinc-950 dark:text-white" />
                    <span class="text-xs text-zinc-500 dark:text-zinc-400">min</span>
                </div>
            </SettingRow>
            <SettingRow title="Per-game times" :description="customCount ? `${customCount} ${customCount === 1 ? 'game has' : 'games have'} its own time, set on the game's page.` : 'Set a different time for one game on its page.'">
                <button v-if="customCount" class="btn-ghost !px-3 !py-1.5 text-xs" @click="settings.gameMinutes = {}">Reset all</button>
            </SettingRow>
            <SettingRow title="Stop games when the app quits" description="Dummy games are stopped when the app quits, so none is left running in the background.">
                <ToggleSwitch label="Stop games when the app quits" v-model="settings.stopOnExit" />
            </SettingRow>
        </section>

        <!-- 2. Your list -->
        <section class="card divide-y divide-zinc-100 dark:divide-zinc-800">
            <h2 class="card-title pb-3">Game list</h2>
            <SettingRow title="Sidebar order" description="Favorites always stay on top.">
                <select v-model="settings.sortOrder" aria-label="Sidebar order"
                    class="h-9 rounded-lg border border-zinc-200 bg-zinc-50 px-2 text-sm text-zinc-900 outline-none focus:border-indigo-500 dark:border-zinc-800 dark:bg-zinc-950 dark:text-white">
                    <option v-for="order in orders" :key="order.value" :value="order.value">{{ order.label }}</option>
                </select>
            </SettingRow>
            <SettingRow title="Refresh the game list" description="Fetch Discord's list of detectable games again, for example when a new game is out.">
                <button class="btn-ghost !px-3 !py-1.5 text-xs" :disabled="isGameListLoading" @click="requestGameListRefresh()">
                    <svg class="mr-1.5 h-3.5 w-3.5" :class="{ 'animate-spin': isGameListLoading }" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M15.312 11.424a5.5 5.5 0 0 1-9.201 2.466l-.312-.311h2.433a.75.75 0 0 0 0-1.5H3.989a.75.75 0 0 0-.75.75v4.242a.75.75 0 0 0 1.5 0v-2.43l.31.31a7 7 0 0 0 11.712-3.138.75.75 0 0 0-1.449-.39Zm1.23-3.723a.75.75 0 0 0 .219-.53V2.929a.75.75 0 0 0-1.5 0V5.36l-.31-.31A7 7 0 0 0 3.239 8.188a.75.75 0 1 0 1.448.389A5.5 5.5 0 0 1 13.89 6.11l.311.31h-2.432a.75.75 0 0 0 0 1.5h4.243a.75.75 0 0 0 .53-.219Z" clip-rule="evenodd"/></svg>
                    {{ isGameListLoading ? 'Refreshing…' : 'Refresh' }}
                </button>
            </SettingRow>
        </section>

        <!-- 3. Steam -->
        <section class="card">
            <h2 class="card-title">Steam library</h2>
            <SettingRow title="Keep Steam entries after a game stops" description="Off: the fake install is removed from your Steam library when the game stops or the app quits. On: it stays so you can run the game again, until you remove it below.">
                <ToggleSwitch label="Keep Steam entries after a game stops" v-model="settings.keepSteamEntries" />
            </SettingRow>
            <h3 class="mb-2 mt-2 border-t border-zinc-100 pt-4 text-sm font-medium text-zinc-900 dark:border-zinc-800 dark:text-white">Entries added by this app</h3>
            <SteamFakesPanel />
        </section>

        <!-- 4. Discord -->
        <section class="card divide-y divide-zinc-100 dark:divide-zinc-800">
            <h2 class="card-title pb-3">Discord</h2>
            <SettingRow title="Status">
                <div class="flex items-center gap-3">
                    <span class="flex items-center gap-1.5 text-xs">
                        <span class="h-2 w-2 rounded-full"
                            :class="discord.status.value === null ? 'bg-zinc-400' : discord.status.value.running ? 'bg-emerald-500' : 'bg-amber-500'"></span>
                        {{ discord.status.value === null ? 'Unknown' : discord.status.value.running ? discord.status.value.clients.join(', ') + ' is running' : 'Not running' }}
                    </span>
                    <button class="btn-ghost !px-3 !py-1.5 text-xs" :disabled="discord.checking.value" @click="discord.check()">Check now</button>
                </div>
            </SettingRow>
            <SettingRow title="Warn me when Discord isn't running" description="Quests only count while the Discord desktop app is open.">
                <ToggleSwitch label="Warn me when Discord isn't running" v-model="settings.checkDiscord" />
            </SettingRow>
            <p class="py-3 text-xs text-zinc-500 dark:text-zinc-400">
                The app can't see Discord's own activity settings. If a quest still doesn't count with Discord open, check in Discord that
                activity sharing / game detection is turned on (User Settings → Activity Settings).
            </p>
        </section>

        <!-- 5. How the app behaves on your PC -->
        <section class="card divide-y divide-zinc-100 dark:divide-zinc-800">
            <h2 class="card-title pb-3">Startup &amp; tray</h2>
            <SettingRow title="Start with Windows" description="Open the app when you sign in to Windows.">
                <ToggleSwitch label="Start with Windows" :model-value="startWithWindows" :disabled="autostartBusy"
                    @update:model-value="applyAutostart" />
                <template #extra>
                    <p v-if="autostartError" class="mt-1 break-words text-xs text-red-500">{{ autostartError }}</p>
                </template>
            </SettingRow>
            <SettingRow title="Start minimized to the tray" description="When Windows starts the app, keep it in the tray instead of opening the window.">
                <ToggleSwitch label="Start minimized to the tray" :model-value="settings.startMinimized" @update:model-value="setStartMinimized" />
            </SettingRow>
            <SettingRow title="Close button hides to the tray" description="Keep running in the background. Use the tray icon to open the window again or to quit.">
                <ToggleSwitch label="Close button hides to the tray" v-model="settings.closeToTray" />
            </SettingRow>
        </section>

        <section class="card divide-y divide-zinc-100 dark:divide-zinc-800">
            <h2 class="card-title pb-3">Appearance</h2>
            <SettingRow title="Theme" description="Follow Windows, or always use light or dark.">
                <div class="inline-flex rounded-lg border border-zinc-200 p-0.5 dark:border-zinc-700" role="radiogroup" aria-label="Theme">
                    <button v-for="option in themes" :key="option.value" role="radio" :aria-checked="themeChoice === option.value"
                        class="rounded-md px-3 py-1 text-xs font-medium transition"
                        :class="themeChoice === option.value ? 'bg-indigo-600 text-white' : 'text-zinc-600 hover:bg-zinc-100 dark:text-zinc-300 dark:hover:bg-zinc-800'"
                        @click="themeChoice = option.value">
                        {{ option.label }}
                    </button>
                </div>
            </SettingRow>
        </section>

        <!-- 6. About the app -->
        <section class="card divide-y divide-zinc-100 dark:divide-zinc-800">
            <h2 class="card-title pb-3">Updates</h2>
            <SettingRow title="Version">
                <div class="flex items-center gap-3">
                    <span class="font-mono text-xs text-zinc-600 dark:text-zinc-300">{{ version || '…' }}</span>
                    <button class="btn-ghost !px-3 !py-1.5 text-xs" :disabled="updates.checking.value" @click="updates.check()">
                        {{ updates.checking.value ? 'Checking…' : 'Check now' }}
                    </button>
                </div>
                <template #extra>
                    <p v-if="updates.error.value" class="mt-1 break-words text-xs text-red-500">{{ updates.error.value }}</p>
                    <p v-else-if="updates.info.value && updates.info.value.update_available" class="mt-1 text-xs text-indigo-600 dark:text-indigo-400">
                        Version {{ updates.info.value.latest }} is available.
                        <button class="underline" @click="updates.openRelease()">View release</button>
                    </p>
                    <p v-else-if="updates.info.value" class="mt-1 text-xs text-emerald-600 dark:text-emerald-400">You're up to date.</p>
                </template>
            </SettingRow>
            <SettingRow title="Check for updates automatically" description="Looks for a newer release on GitHub when the app starts and every few hours. Nothing is downloaded or installed.">
                <ToggleSwitch label="Check for updates automatically" v-model="settings.checkUpdates" />
            </SettingRow>
        </section>

        <section class="card">
            <h2 class="card-title mb-3">Feedback &amp; support</h2>
            <FeedbackSection />
        </section>
    </div>
</template>
