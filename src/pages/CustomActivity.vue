<script setup lang="ts">
import { computed, onUnmounted, reactive, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useGlobalState } from '@/composables/app-state';
import {
    ACTIVITY_EXAMPLES, ACTIVITY_KINDS, MAX_LINE, activityHeadline, activityTitle, buildActivityJson, emptyActivity, normalizeActivities, validateActivity,
    type CustomActivity,
} from '@/composables/custom-activity-model';
import { randomString } from '@/utils/random-string';
import ToggleSwitch from '@/components/ToggleSwitch.vue';

const { addLog, rpcOwner } = useGlobalState();

// ---- Saved presets (kept in the webview's local storage) ----
const STORAGE_KEY = 'discord-quest-completer.activities.v1';

function loadPresets(): CustomActivity[] {
    try {
        return normalizeActivities(JSON.parse(globalThis.localStorage?.getItem(STORAGE_KEY) ?? '[]'));
    } catch {
        return [];
    }
}
const presets = ref<CustomActivity[]>(loadPresets());
watch(presets, (list) => {
    try {
        globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(list));
    } catch {
        // blocked or full: presets just won't persist this time
    }
}, { deep: true });

// ---- The status being edited ----
const draft = reactive<CustomActivity>(emptyActivity());
const error = ref('');
const pending = ref(false); // asked Discord, waiting for the answer
const active = ref(false); // showing on the profile

const kindLabel = computed(() => ACTIVITY_KINDS.find(k => k.value === draft.kind)?.label ?? 'Playing');
const example = computed(() => ACTIVITY_EXAMPLES[draft.kind]);
const detailsLeft = computed(() => MAX_LINE - Array.from(draft.details).length);
const stateLeft = computed(() => MAX_LINE - Array.from(draft.state).length);

function load(preset: CustomActivity) {
    Object.assign(draft, preset);
    error.value = '';
}

function savePreset() {
    const problem = validateActivity(draft);
    if (problem) {
        error.value = problem;
        return;
    }
    error.value = '';
    const existing = presets.value.find(p => p.id === draft.id);
    if (existing) {
        Object.assign(existing, { ...draft });
    } else {
        draft.id = randomString();
        presets.value.push({ ...draft });
    }
}

function saveAsNew() {
    draft.id = '';
    savePreset();
}

function removePreset(preset: CustomActivity) {
    presets.value = presets.value.filter(p => p.id !== preset.id);
    if (draft.id === preset.id) {
        draft.id = '';
    }
}

// ---- Rich Presence ----
// The backend connects in the background and reports with events, shared with the Test RPC button.
const unlisten: UnlistenFn[] = [];
listen('client_connected', () => {
    if (pending.value) {
        pending.value = false;
        active.value = true;
        addLog('info', `Custom activity started: ${kindLabel.value} (app ${draft.appId.trim()})`);
    }
}).then(fn => unlisten.push(fn)).catch(() => {});
listen<{ message: string }>('client_error', (event) => {
    if (pending.value || active.value) {
        pending.value = false;
        active.value = false;
        error.value = event.payload.message;
        addLog('error', `Custom activity failed: ${event.payload.message}`);
        if (rpcOwner.value === 'custom') rpcOwner.value = null;
    }
}).then(fn => unlisten.push(fn)).catch(() => {});
onUnmounted(() => unlisten.forEach(fn => fn()));

// Another Rich Presence (the Test RPC button) took over the one connection.
watch(rpcOwner, (owner) => {
    if (owner !== 'custom') {
        pending.value = false;
        active.value = false;
    }
});

async function start() {
    const problem = validateActivity(draft);
    if (problem) {
        error.value = problem;
        return;
    }
    error.value = '';
    pending.value = true;
    rpcOwner.value = 'custom';
    try {
        await invoke('connect_to_discord_rpc_3', { activity_json: buildActivityJson(draft, Date.now() / 1000) });
    } catch (e) {
        pending.value = false;
        error.value = String(e);
        if (rpcOwner.value === 'custom') rpcOwner.value = null;
    }
}

function stop() {
    emit('event_disconnect');
    pending.value = false;
    active.value = false;
    if (rpcOwner.value === 'custom') rpcOwner.value = null;
    addLog('info', 'Custom activity stopped');
}

function openPortal() {
    invoke('open_link', { key: 'discord_developer_portal' }).catch((e) => { error.value = String(e); });
}

const input = 'h-9 w-full rounded-lg border border-zinc-200 bg-zinc-50 px-3 text-sm text-zinc-900 outline-none transition placeholder:text-zinc-400 focus:border-indigo-500 focus:ring-4 focus:ring-indigo-500/15 dark:border-zinc-800 dark:bg-zinc-950 dark:text-white';
</script>

<template>
    <div class="mx-auto max-w-2xl space-y-4 p-6">
        <div>
            <h1 class="text-lg font-semibold text-zinc-900 dark:text-white">Custom activity</h1>
            <p class="mt-1 text-xs text-zinc-500 dark:text-zinc-400">
                Show your own status on your Discord profile, like "Playing Red Dead Redemption" or "Watching Netflix".
                This is Rich Presence only: it does <span class="font-medium">not</span> complete quests.
            </p>
        </div>

        <section class="card space-y-4">
            <div>
                <label class="mb-1 block text-xs font-medium text-zinc-600 dark:text-zinc-300">Type</label>
                <div class="inline-flex flex-wrap rounded-lg border border-zinc-200 p-0.5 dark:border-zinc-700" role="radiogroup" aria-label="Activity type">
                    <button v-for="kind in ACTIVITY_KINDS" :key="kind.value" role="radio" :aria-checked="draft.kind === kind.value"
                        class="rounded-md px-3 py-1 text-xs font-medium transition"
                        :class="draft.kind === kind.value ? 'bg-indigo-600 text-white' : 'text-zinc-600 hover:bg-zinc-100 dark:text-zinc-300 dark:hover:bg-zinc-800'"
                        @click="draft.kind = kind.value">
                        {{ kind.label }}
                    </button>
                </div>
            </div>

            <div>
                <label class="mb-1 block text-xs font-medium text-zinc-600 dark:text-zinc-300" for="activity-app-id">Application ID</label>
                <input id="activity-app-id" v-model="draft.appId" type="text" inputmode="numeric" spellcheck="false"
                    placeholder="e.g. 1158877933042143272" :class="input + ' font-mono'" />
                <p class="mt-1 text-[11px] text-zinc-500 dark:text-zinc-400">
                    Discord shows the <span class="font-medium">name of this application</span> after "{{ kindLabel }}",
                    for example "{{ activityHeadline(draft.kind, example.name) }}".
                    For any name you like, create your own application named "{{ example.name }}" and paste its ID here.
                    <button class="text-indigo-600 underline dark:text-indigo-400" @click="openPortal()">Open the Developer Portal</button>
                </p>
            </div>

            <div class="grid gap-4 sm:grid-cols-2">
                <div>
                    <label class="mb-1 flex justify-between text-xs font-medium text-zinc-600 dark:text-zinc-300" for="activity-details">
                        Details <span class="font-normal text-zinc-400">{{ detailsLeft }}</span>
                    </label>
                    <input id="activity-details" v-model="draft.details" type="text" :maxlength="MAX_LINE" :placeholder="example.details" :class="input" />
                </div>
                <div>
                    <label class="mb-1 flex justify-between text-xs font-medium text-zinc-600 dark:text-zinc-300" for="activity-state">
                        State <span class="font-normal text-zinc-400">{{ stateLeft }}</span>
                    </label>
                    <input id="activity-state" v-model="draft.state" type="text" :maxlength="MAX_LINE" :placeholder="example.state" :class="input" />
                </div>
            </div>

            <div class="flex items-center justify-between gap-4">
                <div>
                    <div class="text-sm font-medium text-zinc-900 dark:text-white">Show elapsed time</div>
                    <p class="text-xs text-zinc-500 dark:text-zinc-400">Counts up from the moment you start.</p>
                </div>
                <ToggleSwitch label="Show elapsed time" v-model="draft.showTimer" />
            </div>

            <!-- What it will look like -->
            <div class="rounded-xl bg-zinc-100 p-3 text-xs dark:bg-zinc-950">
                <div class="text-[11px] font-semibold uppercase tracking-wider text-zinc-400">Preview</div>
                <div class="mt-1 font-semibold text-zinc-900 dark:text-white">{{ kindLabel }} <span class="font-normal italic text-zinc-500 dark:text-zinc-400">your application's name</span></div>
                <div v-if="draft.details.trim()" class="text-zinc-600 dark:text-zinc-300">{{ draft.details }}</div>
                <div v-if="draft.state.trim()" class="text-zinc-600 dark:text-zinc-300">{{ draft.state }}</div>
                <div v-if="draft.showTimer" class="font-mono text-zinc-400">00:00 elapsed</div>
            </div>

            <p v-if="error" role="alert" class="break-words rounded-lg bg-red-500/10 px-3 py-2 text-xs text-red-500">{{ error }}</p>

            <div class="flex flex-wrap items-center gap-2">
                <button v-if="!active && !pending" class="btn-primary" @click="start()">Start</button>
                <template v-else>
                    <button class="btn-ghost" :disabled="pending" @click="start()">Update</button>
                    <button class="btn-danger" @click="stop()">{{ pending ? 'Cancel' : 'Stop' }}</button>
                </template>
                <button class="btn-ghost" @click="savePreset()">{{ draft.id ? 'Update preset' : 'Save as preset' }}</button>
                <button v-if="draft.id" class="btn-ghost" @click="saveAsNew()">Save as new</button>
                <input v-model="draft.label" type="text" maxlength="60" placeholder="Preset name (optional)" aria-label="Preset name"
                    :class="input + ' !w-48'" />
                <span v-if="pending" class="text-xs text-zinc-500 dark:text-zinc-400">Connecting to Discord…</span>
                <span v-else-if="active" class="flex items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-400">
                    <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-500"></span> Showing on your profile
                </span>
            </div>
        </section>

        <section class="card">
            <h2 class="card-title mb-2">Saved presets</h2>
            <p v-if="presets.length === 0" class="rounded-lg border border-dashed border-zinc-300 px-3 py-4 text-center text-xs text-zinc-500 dark:border-zinc-700 dark:text-zinc-400">
                Nothing saved yet. Fill in a status and press "Save as preset".
            </p>
            <ul v-else class="space-y-2">
                <li v-for="preset in presets" :key="preset.id"
                    class="flex items-center gap-3 rounded-xl border p-3"
                    :class="draft.id === preset.id ? 'border-indigo-500/50 bg-indigo-500/5' : 'border-zinc-200 dark:border-zinc-800'">
                    <div class="min-w-0 flex-1">
                        <div class="truncate text-sm font-medium text-zinc-900 dark:text-white">{{ activityTitle(preset) }}</div>
                        <div class="mt-0.5 flex flex-wrap items-center gap-2 text-[11px] text-zinc-400">
                            <span class="chip">{{ ACTIVITY_KINDS.find(k => k.value === preset.kind)?.label }}</span>
                            <span class="font-mono">{{ preset.appId }}</span>
                        </div>
                    </div>
                    <button class="btn-ghost shrink-0 !px-3 !py-1.5 text-xs" @click="load(preset)">Load</button>
                    <button class="btn-danger shrink-0 !px-3 !py-1.5 text-xs" @click="removePreset(preset)">Delete</button>
                </li>
            </ul>
        </section>
    </div>
</template>
