import { createApp } from 'vue';
import { createPinia } from 'pinia';
import '@fontsource-variable/cairo';
import './assets/styles/design-system.css';
import App from './App.vue';
import router from './router';
import { bootMockDb } from './mocks';
import { buildActionCommands, buildPageCommands, customersProvider, productsProvider, suppliersProvider } from './modules/core/commandPalette/exampleProviders';
import { registerCommands, registerSearchProviders } from './modules/core/controllers/useCommandPalette';
import { useNotificationStore } from './modules/core/controllers/useNotificationStore';
import { initAppearance } from './modules/core/controllers/useAppearance';
import { initTheme } from './modules/core/controllers/useTheme';
import { errorMessage } from './modules/core/controllers/useToast';
import { initAutoBackup } from './modules/settings/services/backupService';

initTheme();
initAppearance();

async function bootstrap() {
  // Load a persisted IndexedDB snapshot if one exists; otherwise `db` stays empty and the router
  // guard sends the first navigation to /welcome (demo data seeds it, or "start company" does).
  await bootMockDb();

  const app = createApp(App);
  const pinia = createPinia();
  app.use(pinia);
  app.use(router);

  // Command palette example providers (docs/v2/14-platform.md §2) — later phases add each
  // module's own `commands.ts` here (or via a plugin-install convention) instead.
  registerCommands([...buildPageCommands(router), ...buildActionCommands(router)]);
  registerSearchProviders([customersProvider, suppliersProvider, productsProvider]);

  // Last-resort handler: anything not caught by a page's ErrorBoundary becomes a toast, never a blank screen.
  app.config.errorHandler = (err, _instance, info) => {
    console.error('[app]', info, err);
    useNotificationStore(pinia).addNotification({ type: 'error', title: 'حدث خطأ غير متوقع', message: errorMessage(err) });
  };

  app.mount('#app');

  // Phase 13a: daily schedule check + on-close hook (docs/v2/14-platform.md §4 "نسخ تلقائي").
  // No-ops until the user turns automatic backup on in Settings → النسخ الاحتياطي.
  void initAutoBackup();
}

void bootstrap();
