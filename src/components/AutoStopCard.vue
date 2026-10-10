<script setup lang="ts">
import { computed } from 'vue';
import { autoStopMinutesFor, MAX_MINUTES, useSettings } from '@/composables/settings';
import { formatCountdown } from '@/utils/time';
import type { Game } from '@/types/types';

const props = defineProps<{ game: Game; remainingMs: number | null }>();

const { settings } = useSettings();

const custom = computed(() => settings.gameMinutes[props.game.id]);
const effective = computed(() => autoStopMinutesFor(settings, props.game.id));

// A blank box means "use the default"; 0 means "never stop this one".
const input = computed({
    get: () => (custom.value === undefined ? '' : String(custom.value)),
    set: (text: string) => {
        const trimmed = String(text ?? '').trim();
        const minutes = Number(trimmed);
        if (trimmed === '' || !Number.isFinite(minutes) || minutes < 0) {
            delete settings.gameMinutes[props.game.id];
        } else {
            settings.gameMinutes[props.game.id] = Math.min(MAX_MINUTES, Math.round(minutes));
        }
    },
});

const summary = computed(() => {
    if (!settings.autoStopEnabled) return 'Auto-stop is turned off in Settings.';
    if (effective.value === 0) return 'This game never stops on its own.';
    const source = custom.value === undefined ? ' (the default from Settings)' : '';
    return `Stops ${effective.value} min after it starts${source}.`;
});
</script>

<template>
    <div class="card">
        <div class="mb-1 flex items-center justify-between">
            <h3 class="text-sm font-medium text-zinc-900 dark:text-white">Auto-stop</h3>
            <span v-if="remainingMs !== null" class="chip !bg-indigo-500/15 !text-indigo-600 dark:!text-indigo-400">
                stops in {{ formatCountdown(remainingMs) }}
            </span>
        </div>
        <p class="mb-3 text-xs text-zinc-500 dark:text-zinc-400">{{ summary }}</p>
        <div class="flex items-center gap-2">
            <label class="text-xs font-medium text-zinc-600 dark:text-zinc-300" :for="`autostop-${game.id}`">Stop after</label>
            <input :id="`autostop-${game.id}`" v-model="input" type="number" min="0" :max="MAX_MINUTES" inputmode="numeric"
                :placeholder="String(settings.autoStopMinutes)" :disabled="!settings.autoStopEnabled"
                class="h-9 w-24 rounded-lg border border-zinc-200 bg-zinc-50 px-3 text-sm text-zinc-900 outline-none transition focus:border-indigo-500 focus:ring-4 focus:ring-indigo-500/15 disabled:opacity-50 dark:border-zinc-800 dark:bg-zinc-950 dark:text-white" />
            <span class="text-xs text-zinc-500 dark:text-zinc-400">min</span>
            <button v-if="custom !== undefined" class="btn-ghost ml-auto !px-3 !py-1.5 text-xs" @click="input = ''">Use default</button>
        </div>
        <p class="mt-2 text-[11px] text-zinc-400">Leave blank for the default, or enter 0 so this game is never stopped automatically.</p>
    </div>
</template>
