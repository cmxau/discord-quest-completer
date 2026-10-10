<script setup lang="ts">
defineProps<{ title: string; description?: string }>();
</script>

<template>
    <div class="flex items-center justify-between gap-4 py-3">
        <div class="min-w-0">
            <div class="text-sm font-medium text-zinc-900 dark:text-white">{{ title }}</div>
            <p v-if="description" class="mt-0.5 text-xs text-zinc-500 dark:text-zinc-400">{{ description }}</p>
            <slot name="extra"></slot>
        </div>
        <div class="shrink-0"><slot></slot></div>
    </div>
</template>
