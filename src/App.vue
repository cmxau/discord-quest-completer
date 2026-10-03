<script setup lang="ts">
import MainLayout from './components/MainLayout.vue';
import { Pages, useGlobalState } from './composables/app-state';
import HomeView from './pages/HomeView.vue';
import Diagnostics from './pages/Diagnostics.vue';

const appState = useGlobalState();
const { page } = appState;

</script>

<template>
  <MainLayout>
    <HomeView v-show="page === Pages.HOME"/>
    <Diagnostics v-show="page === Pages.DIAGNOSTICS"/>
  </MainLayout>
</template>

<style>
/* Global styles are managed by Tailwind CSS */
</style>