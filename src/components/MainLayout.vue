<script setup lang="ts">
import { computed } from 'vue';
import { Pages, useGlobalState } from '@/composables/app-state';
import { useDark, useToggle } from '@vueuse/core';

// Follows the OS theme until the user toggles it; the choice is then remembered.
const isDark = useDark({ selector: 'html', valueDark: 'dark', valueLight: '' });
const toggleDark = useToggle(isDark);

const appState = useGlobalState();
const { page, setPage, isGameListLoading, requestGameListRefresh } = appState;

// The terminal icon opens Diagnostics; clicking it again returns to the main page.
const isDiagnostics = computed(() => page.value === Pages.DIAGNOSTICS);
function toggleDiagnostics() {
  setPage(isDiagnostics.value ? Pages.HOME : Pages.DIAGNOSTICS);
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
            :title="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
            :aria-label="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
            @click="toggleDark()"
          >
            <svg v-if="isDark" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path d="M10 2a.75.75 0 0 1 .75.75v1.5a.75.75 0 0 1-1.5 0v-1.5A.75.75 0 0 1 10 2Zm0 13a.75.75 0 0 1 .75.75v1.5a.75.75 0 0 1-1.5 0v-1.5A.75.75 0 0 1 10 15Zm7-5a.75.75 0 0 1-.75.75h-1.5a.75.75 0 0 1 0-1.5h1.5A.75.75 0 0 1 17 10ZM5 10a.75.75 0 0 1-.75.75h-1.5a.75.75 0 0 1 0-1.5h1.5A.75.75 0 0 1 5 10Zm9.596-4.596a.75.75 0 0 1 0 1.06l-1.06 1.061a.75.75 0 1 1-1.061-1.06l1.06-1.061a.75.75 0 0 1 1.061 0ZM7.525 12.475a.75.75 0 0 1 0 1.06l-1.06 1.061a.75.75 0 1 1-1.061-1.06l1.06-1.061a.75.75 0 0 1 1.061 0Zm7.07 2.121a.75.75 0 0 1-1.06 0l-1.061-1.06a.75.75 0 1 1 1.06-1.061l1.061 1.06a.75.75 0 0 1 0 1.061ZM7.525 7.525a.75.75 0 0 1-1.06 0l-1.061-1.06a.75.75 0 0 1 1.06-1.061l1.061 1.06a.75.75 0 0 1 0 1.061ZM10 6.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7Z"/></svg>
            <svg v-else class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M7.455 2.004a.75.75 0 0 1 .26.77 7 7 0 0 0 9.958 7.967.75.75 0 0 1 1.067.853A8.5 8.5 0 1 1 6.647 1.921a.75.75 0 0 1 .808.083Z" clip-rule="evenodd"/></svg>
          </button>

          <button
            class="btn-ghost !px-3 !py-1.5 text-xs"
            :disabled="isGameListLoading"
            title="Refetch the game list"
            @click="requestGameListRefresh()"
          >
            <svg class="mr-1.5 h-3.5 w-3.5" :class="{ 'animate-spin': isGameListLoading }" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M15.312 11.424a5.5 5.5 0 0 1-9.201 2.466l-.312-.311h2.433a.75.75 0 0 0 0-1.5H3.989a.75.75 0 0 0-.75.75v4.242a.75.75 0 0 0 1.5 0v-2.43l.31.31a7 7 0 0 0 11.712-3.138.75.75 0 0 0-1.449-.39Zm1.23-3.723a.75.75 0 0 0 .219-.53V2.929a.75.75 0 0 0-1.5 0V5.36l-.31-.31A7 7 0 0 0 3.239 8.188a.75.75 0 1 0 1.448.389A5.5 5.5 0 0 1 13.89 6.11l.311.31h-2.432a.75.75 0 0 0 0 1.5h4.243a.75.75 0 0 0 .53-.219Z" clip-rule="evenodd"/></svg>
            Refresh list
          </button>
        </div>
      </div>
    </header>

    <main class="min-h-0 grow overflow-y-auto">
      <slot></slot>
    </main>
  </div>
</template>
