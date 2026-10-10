import { createGlobalState, useIntervalFn } from '@vueuse/core';
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useSettings } from '@/composables/settings';

export interface UpdateInfo {
    current: string;
    /** The newest published release, e.g. "26.10.2"; null when there is no release yet. */
    latest: string | null;
    update_available: boolean;
    /** The release's page on GitHub. */
    url: string | null;
}

const DISMISSED_KEY = 'discord-quest-completer.update-dismissed';
const RECHECK_EVERY_MS = 6 * 60 * 60 * 1000;

function readDismissed(): string {
    try {
        return globalThis.localStorage?.getItem(DISMISSED_KEY) ?? '';
    } catch {
        return '';
    }
}

/** Looks for a newer release on GitHub: once shortly after start, then every few hours. */
export const useUpdates = createGlobalState(() => {
    const { settings } = useSettings();
    const info = ref<UpdateInfo | null>(null);
    const checking = ref(false);
    const error = ref('');
    const dismissedVersion = ref(readDismissed());

    /** There is a newer release the user hasn't dismissed. */
    const available = computed(() => !!info.value?.update_available && info.value.latest !== dismissedVersion.value);

    async function check() {
        if (checking.value) {
            return;
        }
        checking.value = true;
        error.value = '';
        try {
            info.value = await invoke<UpdateInfo>('check_for_update');
        } catch (e) {
            error.value = String(e);
        } finally {
            checking.value = false;
        }
    }

    function dismiss() {
        dismissedVersion.value = info.value?.latest ?? '';
        try {
            globalThis.localStorage?.setItem(DISMISSED_KEY, dismissedVersion.value);
        } catch {
            // blocked storage: it just shows again next time
        }
    }

    async function openRelease() {
        if (info.value?.url) {
            await invoke('open_release_page', { url: info.value.url });
        }
    }

    setTimeout(() => { if (settings.checkUpdates) check(); }, 4000);
    useIntervalFn(() => { if (settings.checkUpdates) check(); }, RECHECK_EVERY_MS);

    return { info, checking, error, available, check, dismiss, openRelease };
});
