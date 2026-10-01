import { createApp } from "vue";
import { createPinia } from "pinia";
import "@fontsource-variable/cairo";
import "./shared/styles/base.css";
import App from "./App.vue";
import router from "./router";

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
