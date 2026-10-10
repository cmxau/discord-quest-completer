<script setup lang="ts">
import { ref } from 'vue';
import { useGlobalState } from '@/composables/app-state';
import { openFeedback, type FeedbackKind } from '@/composables/feedback';
import { invoke } from '@tauri-apps/api/core';

// Opens a pre-filled bug report or feature request on GitHub in the browser. Nothing is sent until
// the user reviews the page there and submits it.
const { logs, selectedGameLabel, addLog } = useGlobalState();

const busy = ref(false);
const error = ref('');

async function star() {
    error.value = '';
    try {
        await invoke('open_link', { key: 'repo' });
    } catch (e) {
        error.value = `Couldn't open the browser: ${e}`;
    }
}

async function send(kind: FeedbackKind) {
    busy.value = true;
    error.value = '';
    try {
        await openFeedback(kind, { game: selectedGameLabel.value, logs: logs.value });
    } catch (e) {
        error.value = `Couldn't open the browser: ${e}`;
        addLog('error', `Feedback link failed: ${e}`);
    } finally {
        busy.value = false;
    }
}
</script>

<template>
    <div>
        <div class="grid gap-2 sm:grid-cols-2">
            <button class="flex items-start gap-3 rounded-xl border border-zinc-200 p-3 text-left transition hover:bg-zinc-50 disabled:opacity-60 dark:border-zinc-800 dark:hover:bg-zinc-800/60"
                :disabled="busy" @click="send('bug')">
                <svg class="mt-0.5 h-4 w-4 shrink-0 text-red-500" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M7 6.5V6a3 3 0 0 1 6 0v.5M5 8.5h10v3.2a5 5 0 0 1-10 0V8.5ZM10 9v7M2.5 9.5 5 10.5M17.5 9.5 15 10.5M3 14.5l2.4-1.2M17 14.5l-2.4-1.2"/></svg>
                <span>
                    <span class="block text-sm font-medium text-zinc-900 dark:text-white">Report a bug</span>
                    <span class="block text-xs text-zinc-500 dark:text-zinc-400">Opens a pre-filled issue on GitHub</span>
                </span>
            </button>
            <button class="flex items-start gap-3 rounded-xl border border-zinc-200 p-3 text-left transition hover:bg-zinc-50 disabled:opacity-60 dark:border-zinc-800 dark:hover:bg-zinc-800/60"
                :disabled="busy" @click="send('feature')">
                <svg class="mt-0.5 h-4 w-4 shrink-0 text-amber-500" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M7.5 14.5h5M8 17h4M10 2.5a5 5 0 0 0-3 9c.7.6 1 1.2 1 2h4c0-.8.3-1.4 1-2a5 5 0 0 0-3-9Z"/></svg>
                <span>
                    <span class="block text-sm font-medium text-zinc-900 dark:text-white">Request a feature</span>
                    <span class="block text-xs text-zinc-500 dark:text-zinc-400">Suggest an idea or improvement</span>
                </span>
            </button>
        </div>
        <p v-if="error" class="mt-3 break-words rounded-lg bg-red-500/10 px-3 py-2 text-xs text-red-500">{{ error }}</p>
        <p class="mt-3 text-xs text-zinc-500 dark:text-zinc-400">
            You review everything on GitHub before submitting. Personal folder names are removed from the log.
        </p>

        <div class="mt-4 flex items-center justify-between gap-3 rounded-xl bg-amber-500/10 px-3 py-2.5">
            <p class="text-xs text-amber-700 dark:text-amber-400">Enjoying the app? A star on GitHub helps other people find it.</p>
            <button class="btn-ghost shrink-0 !px-3 !py-1.5 text-xs" @click="star()">
                <svg class="mr-1.5 h-3.5 w-3.5 text-amber-500" viewBox="0 0 20 20" fill="currentColor"><path d="M10 2.5l2.2 4.6 5 .7-3.6 3.5.9 5-4.5-2.4-4.5 2.4.9-5L2.8 7.8l5-.7L10 2.5Z"/></svg>
                Star on GitHub
            </button>
        </div>
    </div>
</template>
