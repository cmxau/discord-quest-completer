<script setup lang="ts">
defineProps<{ modelValue: boolean; disabled?: boolean; label: string }>();
defineEmits<{ 'update:modelValue': [value: boolean] }>();
</script>

<template>
    <button type="button" role="switch" :aria-checked="modelValue" :aria-label="label" :disabled="disabled"
        class="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition focus:outline-none focus-visible:ring-4 focus-visible:ring-indigo-500/25"
        :class="modelValue ? 'bg-indigo-600' : 'bg-zinc-300 dark:bg-zinc-700'"
        @click="$emit('update:modelValue', !modelValue)">
        <span class="inline-block h-5 w-5 transform rounded-full bg-white shadow transition"
            :class="modelValue ? 'translate-x-[1.375rem]' : 'translate-x-0.5'"></span>
    </button>
</template>
