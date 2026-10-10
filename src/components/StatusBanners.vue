<script setup lang="ts">
import { computed, ref } from 'vue';
import { useSettings } from '@/composables/settings';
import { useDiscordStatus } from '@/composables/discord-status';
import { useUpdates } from '@/composables/updates';

const { settings } = useSettings();
const discord = useDiscordStatus();
const updates = useUpdates();

const showDiscord = computed(() =>
    settings.checkDiscord && discord.status.value !== null && !discord.status.value.running && !discord.dismissed.value,
);

const openError = ref('');
async function openRelease() {
    openError.value = '';
    try {
        await updates.openRelease();
    } catch (e) {
        openError.value = String(e);
    }
}
</script>

<template>
    <div v-if="showDiscord || updates.available.value">
        <div v-if="showDiscord" role="alert"
            class="flex items-center gap-3 border-b border-amber-500/30 bg-amber-500/10 px-4 py-2 text-xs text-amber-700 dark:text-amber-400">
            <svg class="h-4 w-4 shrink-0" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495ZM10 6a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 10 6Zm0 9a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z" clip-rule="evenodd"/></svg>
            <span class="min-w-0 flex-1">
                <span class="font-semibold">Discord isn't running.</span>
                Quests only count while the Discord desktop app is open. Start it, then launch the game.
            </span>
            <button class="shrink-0 rounded-md px-2 py-1 font-medium hover:bg-amber-500/20" :disabled="discord.checking.value" @click="discord.check()">Check again</button>
            <button class="shrink-0 rounded-md p-1 hover:bg-amber-500/20" title="Dismiss" aria-label="Dismiss" @click="discord.dismissed.value = true">
                <svg class="h-3.5 w-3.5" viewBox="0 0 20 20" fill="currentColor"><path d="M6.28 5.22a.75.75 0 0 0-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 1 0 1.06 1.06L10 11.06l3.72 3.72a.75.75 0 1 0 1.06-1.06L11.06 10l3.72-3.72a.75.75 0 0 0-1.06-1.06L10 8.94 6.28 5.22Z"/></svg>
            </button>
        </div>

        <div v-if="updates.available.value" role="status"
            class="flex items-center gap-3 border-b border-indigo-500/30 bg-indigo-500/10 px-4 py-2 text-xs text-indigo-700 dark:text-indigo-300">
            <span class="min-w-0 flex-1">
                <span class="font-semibold">Version {{ updates.info.value?.latest }} is available.</span>
                You're on {{ updates.info.value?.current }}.
                <span v-if="openError" class="text-red-500">{{ openError }}</span>
            </span>
            <button class="shrink-0 rounded-md px-2 py-1 font-medium hover:bg-indigo-500/20" @click="openRelease()">View release</button>
            <button class="shrink-0 rounded-md p-1 hover:bg-indigo-500/20" title="Dismiss" aria-label="Dismiss" @click="updates.dismiss()">
                <svg class="h-3.5 w-3.5" viewBox="0 0 20 20" fill="currentColor"><path d="M6.28 5.22a.75.75 0 0 0-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 1 0 1.06 1.06L10 11.06l3.72 3.72a.75.75 0 1 0 1.06-1.06L11.06 10l3.72-3.72a.75.75 0 0 0-1.06-1.06L10 8.94 6.28 5.22Z"/></svg>
            </button>
        </div>
    </div>
</template>
