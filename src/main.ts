import { createApp } from 'vue';
import { createPinia } from 'pinia';
import '@fontsource-variable/cairo';
import './assets/styles/design-system.css';
import App from './App.vue';
import router from './router';
import { initTheme } from './modules/core/controllers/useTheme';
import { useNotificationStore } from './modules/core/controllers/useNotificationStore';
import { errorMessage } from './modules/core/controllers/useToast';

initTheme();

const app = createApp(App);
const pinia = createPinia();
app.use(pinia);
app.use(router);

// Last-resort handler: anything not caught by a page's ErrorBoundary becomes a toast, never a blank screen.
app.config.errorHandler = (err, _instance, info) => {
  console.error('[app]', info, err);
  useNotificationStore(pinia).addNotification({ type: 'error', title: 'حدث خطأ غير متوقع', message: errorMessage(err) });
};

app.mount('#app');
