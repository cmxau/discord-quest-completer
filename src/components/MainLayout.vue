<script setup lang="ts">
import { computed } from 'vue';
import StatusBanners from '@/components/StatusBanners.vue';
import { useTheme } from '@/composables/settings';
import { Pages, useGlobalState } from '@/composables/app-state';

// Applies the saved theme (light, dark or follow Windows) at startup; it is changed in Settings.
useTheme();

const { page, setPage } = useGlobalState();

// The terminal icon opens Diagnostics and the gear opens Settings; clicking the open one again
// returns to the main page, and the house always goes there.
const isHome = computed(() => page.value === Pages.HOME);
const isDiagnostics = computed(() => page.value === Pages.DIAGNOSTICS);
const isSettings = computed(() => page.value === Pages.SETTINGS);
function toggleDiagnostics() {
  setPage(isDiagnostics.value ? Pages.HOME : Pages.DIAGNOSTICS);
}
function toggleSettings() {
  setPage(isSettings.value ? Pages.HOME : Pages.SETTINGS);
}
</script>

<template>
  <div class="flex h-dvh flex-col overflow-hidden bg-zinc-50 dark:bg-zinc-950">
    <header class="z-30 border-b border-zinc-200 bg-white/80 backdrop-blur dark:border-zinc-800 dark:bg-zinc-950/80">
      <div class="flex items-center justify-between gap-3 px-4 py-2.5">
        <span class="text-sm font-semibold tracking-tight text-zinc-900 dark:text-white">Quest Completer</span>

        <div class="flex items-center gap-2">
          <button
            class="btn-ghost !px-2 !py-1.5"
            :class="isHome ? '!border-indigo-500/50 !bg-indigo-500/10 !text-indigo-600 dark:!text-indigo-400' : ''"
            title="Home"
            aria-label="Go to the games page"
            :aria-current="isHome ? 'page' : undefined"
            @click="setPage(Pages.HOME)"
          >
            <svg class="h-4 w-4" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9.2 10 3l7 6.2V16a1 1 0 0 1-1 1h-3.5v-4.5h-5V17H4a1 1 0 0 1-1-1V9.2Z"/></svg>
          </button>

          <button
            class="btn-ghost !px-2 !py-1.5"
            :class="isDiagnostics ? '!border-indigo-500/50 !bg-indigo-500/10 !text-indigo-600 dark:!text-indigo-400' : ''"
            :title="isDiagnostics ? 'Back to games' : 'Diagnostics'"
            :aria-label="isDiagnostics ? 'Back to games' : 'Open diagnostics'"
            :aria-pressed="isDiagnostics"
            @click="toggleDiagnostics()"
          >
            <svg class="h-4 w-4" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="2.5" y="3.5" width="15" height="13" rx="2.5"/><path d="m6 8 2.5 2L6 12M10.5 12H14"/></svg>
          </button>

          <button
            class="btn-ghost !px-2 !py-1.5"
            :class="isSettings ? '!border-indigo-500/50 !bg-indigo-500/10 !text-indigo-600 dark:!text-indigo-400' : ''"
            :title="isSettings ? 'Back to games' : 'Settings'"
            :aria-label="isSettings ? 'Back to games' : 'Open settings'"
            :aria-pressed="isSettings"
            @click="toggleSettings()"
          >
            <svg class="h-4 w-4" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="10" cy="10" r="2.5"/><path d="M8.6 2.8h2.8l.4 1.9 1.3.6 1.7-1 2 2-1 1.7.6 1.3 1.9.4v2.8l-1.9.4-.6 1.3 1 1.7-2 2-1.7-1-1.3.6-.4 1.9H8.6l-.4-1.9-1.3-.6-1.7 1-2-2 1-1.7-.6-1.3-1.9-.4V8.6l1.9-.4.6-1.3-1-1.7 2-2 1.7 1 1.3-.6.4-1.9Z"/></svg>
          </button>
        </div>
      </div>
    </header>

    <StatusBanners />

    <main class="min-h-0 grow overflow-y-auto">
      <slot></slot>
    </main>
  </div>
</template>
