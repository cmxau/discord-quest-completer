import { createGlobalState, useIntervalFn } from '@vueuse/core';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useSettings } from '@/composables/settings';

export interface DiscordStatus {
    running: boolean;
    /** Which Discord apps were found, e.g. ["Discord"] or ["Discord", "Discord PTB"]. */
    clients: string[];
}

const CHECK_EVERY_MS = 15_000;

/** Whether the Discord desktop app is running. Quests only count while it is, so the app warns when it isn't. */
export const useDiscordStatus = createGlobalState(() => {
    const { settings } = useSettings();
    const status = ref<DiscordStatus | null>(null);
    const checking = ref(false);
    /** The warning was dismissed; it comes back after Discord has been seen running and then closes again. */
    const dismissed = ref(false);

    async function check() {
        if (checking.value) {
            return;
        }
        checking.value = true;
        try {
            status.value = await invoke<DiscordStatus>('discord_status');
            if (status.value.running) {
                dismissed.value = false;
            }
        } catch {
            status.value = null; // can't tell: show nothing rather than a wrong warning
        } finally {
            checking.value = false;
        }
    }

    useIntervalFn(() => { if (settings.checkDiscord) check(); }, CHECK_EVERY_MS, { immediateCallback: true });

    return { status, checking, dismissed, check };
});
