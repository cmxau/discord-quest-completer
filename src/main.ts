import { createApp } from "vue";
import '@/theme/style.css'
import App from "./App.vue";
import { isMac } from '@/constants/constants';

// Lets CSS (the `mac:` variant) make room for the macOS traffic lights.
if (isMac) document.documentElement.classList.add('macos');

createApp(App).mount("#app");
