<template>
    <div class="mx-auto max-w-4xl space-y-4 p-6">
        <div class="card flex items-center justify-between gap-4">
            <div>
                <h2 class="card-title">Discord connection test</h2>
                <p class="mt-0.5 flex items-center gap-1.5 text-xs text-zinc-500 dark:text-zinc-400">
                    <span class="h-1.5 w-1.5 rounded-full" :class="isConnected ? 'animate-pulse bg-emerald-500' : 'bg-zinc-400'"></span>
                    {{ isConnected ? 'Connected' : 'Disconnected' }}
                </p>
            </div>
            <button :class="isConnected ? 'btn-danger' : 'btn-primary'" @click="discordTest">
                {{ isConnected ? 'Disconnect' : 'Connect' }}
            </button>
        </div>

        <!-- Logs Section -->
        <div class="card text-zinc-700 dark:text-zinc-300">
            <div class="mb-2 flex items-center justify-between">
                <h2 class="card-title">Logs</h2>
                <button class="btn-ghost !px-3 !py-1 text-xs" @click="clearLogs">Clear logs</button>
            </div>

            <div class="max-h-64 overflow-y-auto p-2 rounded">
                <!-- <div v-if="logs.length === 0" class="text-gray-400">No logs available.</div>
                <ul v-else class="list-none">
                    <li v-for="(log, index) in logs" :key="index" class="text-sm text-gray-400">{{ log }}</li>
                </ul> -->
                <div v-if="logs.length === 0" class="text-gray-400">No logs available.</div>
                <ul v-else class="list-none">
                    <li v-for="(log, index) in logs" :key="index" class="text-sm">
                        <span class="text-gray-500">[{{ new Date(log.timestamp).toLocaleString() }}]</span>
                        <span :class="{
                            'text-blue-400': log.type === 'info',
                            'text-red-400': log.type === 'error',
                            'text-yellow-400': log.type === 'warning',
                            'text-green-400': log.type === 'debug'
                        }">
                            [{{ log.type.toUpperCase() }}]
                        </span>
                        <span class="ml-1">{{ log.message }}</span>
                    </li>
                </ul>
            </div>


        </div>
    </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { useGlobalState } from '@/composables/app-state';

const ActivityKind = {
    Playing: 0,
    Listening: 2,
    Watching: 3,
    Competing: 5
} as const;

const isConnected = ref(false);

const { logs, addLog, clearLogs } = useGlobalState();

function discordTest() {

    // The "Playing <name>" label comes from this application's registered name (Counter-Strike 2).
    const appIdCode = '1158877933042143272';

    if (isConnected.value) {
        console.log('Disconnecting from Discord');
        emit('event_disconnect');
        isConnected.value = false;
        return;
    }

    invoke('connect_to_discord_rpc_3', {
        activity_json: JSON.stringify({
            app_id: appIdCode,
            details: 'Competitive',
            state: 'In a match',
            activity_kind: ActivityKind.Playing,
            timestamp: createAgoTimestamp('25m')
        }),
    });
    isConnected.value = true;
}

// function to create timestamp behind current time.
// example: input is `4h 30m` means timestamp should start from 4 hours and 30 minutes behind current time.
function createAgoTimestamp(input: string) {
    const time = input.split(' ');
    let hours = 0;
    let minutes = 0;

    for (let i = 0; i < time.length; i++) {
        if (time[i].includes('h')) {
            hours = parseInt(time[i]);
        } else if (time[i].includes('m')) {
            minutes = parseInt(time[i]);
        }
    }

    const date = new Date();
    date.setHours(date.getHours() - hours);
    date.setMinutes(date.getMinutes() - minutes);

    return Math.floor(date.getTime() / 1000);
}


onMounted(() => {

})

</script>

<style scoped></style>