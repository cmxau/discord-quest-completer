import { createGlobalState, useColorMode } from '@vueuse/core';
import { reactive, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { normalizeSettings, type Settings } from '@/composables/settings-model';

export * from '@/composables/settings-model';

const STORAGE_KEY = 'discord-quest-completer.settings.v1';

function load(): Settings {
    try {
        return normalizeSettings(JSON.parse(globalThis.localStorage?.getItem(STORAGE_KEY) ?? 'null'));
    } catch {
        return normalizeSettings(null); // unreadable or blocked storage: defaults
    }
}

function save(settings: Settings) {
    try {
        globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(settings));
    } catch {
        // blocked or full: the settings just won't persist this time
    }
}

export const useSettings = createGlobalState(() => {
    const settings = reactive<Settings>(load());

    watch(settings, () => save(settings), { deep: true });

    // The tray, the quit handler and the Steam cleanup live in the backend, which needs to know these.
    watch(
        () => [settings.closeToTray, settings.stopOnExit, settings.keepSteamEntries] as const,
        ([closeToTray, stopOnExit, keepSteamEntries]) => {
            invoke('set_behavior', {
                close_to_tray: closeToTray,
                stop_on_exit: stopOnExit,
                keep_steam_entries: keepSteamEntries,
            }).catch(() => {});
        },
        { immediate: true },
    );

    return { settings };
});

/**
 * Light / dark / follow Windows. Shares the browser storage key the old header toggle used, so the
 * theme people already chose is kept. Call it once at startup so the theme is applied.
 */
export const useTheme = createGlobalState(() =>
    useColorMode({ selector: 'html', attribute: 'class', modes: { light: '', dark: 'dark' } }),
);
