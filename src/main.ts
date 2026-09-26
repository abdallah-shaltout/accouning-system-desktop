import { createApp } from 'vue';
import { createPinia } from 'pinia';
import '@fontsource-variable/cairo';
import './assets/styles/design-system.css';
import App from './App.vue';
import router from './router';
import { bootMockDb } from './mocks';
import { buildActionCommands, buildPageCommands, customersProvider, productsProvider, suppliersProvider } from './modules/core/commandPalette/exampleProviders';
import { registerCommands, registerSearchProviders } from './modules/core/controllers/useCommandPalette';
// v2 phase 13b (docs/v2/14-platform.md §2): every module's own search provider + context commands,
// completing the shell Phase 0 built with just the pages/actions/customers/suppliers/products examples.
import * as invoiceCommands from './modules/invoices/commands';
import * as purchaseCommands from './modules/purchases/commands';
import * as voucherCommands from './modules/vouchers/commands';
import * as accountingCommands from './modules/accounting/commands';
import * as reportCommands from './modules/reports/commands';
import * as settingsCommands from './modules/settings/commands';
import * as approvalCommands from './modules/approvals/commands';
import * as diagnosticsCommands from './modules/diagnostics/commands';
import { useNotificationStore } from './modules/core/controllers/useNotificationStore';
import { initAppearance } from './modules/core/controllers/useAppearance';
import { registerDefaultShortcuts } from './modules/core/controllers/useKeybindings';
import { initTheme } from './modules/core/controllers/useTheme';
import { errorMessage } from './modules/core/controllers/useToast';
import { initAutoBackup } from './modules/settings/services/backupService';
import { initDiagnostics } from './modules/diagnostics/services/diagnosticsService';
import { fingerprintOf, log } from './modules/diagnostics/services/logService';

initTheme();
initAppearance();
registerDefaultShortcuts();
initDiagnostics();

// Last-resort browser-level nets: anything that never reaches Vue's errorHandler (a script error
// during boot, a rejected promise nobody awaited) still lands on the `error` channel (18.B1).
window.addEventListener('error', (e) => {
  log.error('window.onerror', e.message, e.error instanceof Error ? e.error : new Error(e.message));
});
window.addEventListener('unhandledrejection', (e) => {
  const reason = e.reason;
  log.error('window.unhandledrejection', reason instanceof Error ? reason.message : String(reason), reason instanceof Error ? reason : new Error(String(reason)));
});

async function bootstrap() {
  // Load a persisted IndexedDB snapshot if one exists; otherwise `db` stays empty and the router
  // guard sends the first navigation to /welcome (demo data seeds it, or "start company" does).
  await bootMockDb();

  const app = createApp(App);
  const pinia = createPinia();
  app.use(pinia);
  app.use(router);

  // Command palette (docs/v2/14-platform.md §2): Phase 0's shell + pages/actions/customers/
  // suppliers/products examples, plus every other module's own `commands.ts` (Phase 13b).
  registerCommands([
    ...buildPageCommands(router),
    ...buildActionCommands(router),
    ...invoiceCommands.commands,
    ...invoiceCommands.buildContextCommands(router),
    ...purchaseCommands.commands,
    ...purchaseCommands.buildContextCommands(router),
    ...voucherCommands.commands,
    ...voucherCommands.buildContextCommands(router),
    ...accountingCommands.commands,
    ...accountingCommands.buildContextCommands(router),
    ...approvalCommands.commands,
  ]);
  registerSearchProviders([
    customersProvider,
    suppliersProvider,
    productsProvider,
    ...invoiceCommands.searchProviders,
    ...purchaseCommands.searchProviders,
    ...voucherCommands.searchProviders,
    ...accountingCommands.searchProviders,
    ...reportCommands.searchProviders,
    ...settingsCommands.searchProviders,
    ...diagnosticsCommands.searchProviders,
  ]);

  // Last-resort handler: anything not caught by a page's ErrorBoundary becomes a toast, never a blank screen.
  app.config.errorHandler = (err, _instance, info) => {
    console.error('[app]', info, err);
    const errorObj = err instanceof Error ? err : new Error(String(err));
    log.error('vue.errorHandler', info, errorObj);
    // Short fingerprint code (18.B6) so a non-technical user can read it to support over the
    // phone: "رمز الخطأ: E-7F3A" — the first 4 hex chars of the same fingerprint the ledger uses.
    const code = fingerprintOf(errorObj.name, errorObj.message).slice(0, 4).toUpperCase();
    useNotificationStore(pinia).addNotification({
      type: 'error',
      title: 'حدث خطأ غير متوقع',
      message: `${errorMessage(err)} — رمز الخطأ: E-${code}`,
    });
  };

  app.mount('#app');

  // Phase 13a: daily schedule check + on-close hook (docs/v2/14-platform.md §4 "نسخ تلقائي").
  // No-ops until the user turns automatic backup on in Settings → النسخ الاحتياطي.
  void initAutoBackup();
}

void bootstrap();
