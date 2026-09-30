import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';
import DefaultLayout from '@/modules/core/components/layout/DefaultLayout.vue';
import { isFreshInstall } from '@/modules/users/services/authService';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { APP_NAME_AR } from '@/modules/core/helpers/brand';
import { usesRust } from '@/modules/core/services/backend';
import { deviceStateForNavigation, mustHoldForDatabase } from '@/modules/setup/services/deviceService';
import coreRoutes from '@/modules/core/routes';
import usersRoutes from '@/modules/users/routes';
import productsRoutes from '@/modules/products/routes';
import partiesRoutes from '@/modules/parties/routes';
import accountingRoutes from '@/modules/accounting/routes';
import invoicesRoutes from '@/modules/invoices/routes';
import purchasesRoutes from '@/modules/purchases/routes';
import paymentsRoutes from '@/modules/payments/routes';
import reportsRoutes from '@/modules/reports/routes';
import settingsRoutes from '@/modules/settings/routes';
// v2 phase 10 (docs/v2/11-journal-dashboard-insights.md Part C): analytics tabs.
import analyticsRoutes from '@/modules/analytics/routes';
// v2 phase 8 (docs/v2/09-purchases-payments-expenses.md §4-§5): new expenses + vouchers modules.
import expensesRoutes from '@/modules/expenses/routes';
import vouchersRoutes from '@/modules/vouchers/routes';
// v2 phase 5 (docs/v2/05-onboarding.md): the setup wizard + standalone opening-balances page.
import setupRoutes from '@/modules/setup/routes';
// v2 phase 13b (docs/v2/14-platform.md §6): the async manager-approvals queue.
import approvalsRoutes from '@/modules/approvals/routes';

const moduleRoutes: RouteRecordRaw[] = [
  ...coreRoutes,
  ...usersRoutes,
  ...productsRoutes,
  ...partiesRoutes,
  ...accountingRoutes,
  ...invoicesRoutes,
  ...purchasesRoutes,
  ...paymentsRoutes,
  ...reportsRoutes,
  ...settingsRoutes,
  ...analyticsRoutes,
  ...expensesRoutes,
  ...vouchersRoutes,
  ...setupRoutes,
  ...approvalsRoutes,
];

/** Pages marked `layout: 'blank'` render full-screen; everything else sits inside the app shell. */
const blankRoutes = moduleRoutes.filter((r) => r.meta?.layout === 'blank');
const shellRoutes = moduleRoutes
  .filter((r) => r.meta?.layout !== 'blank')
  .map((r) => ({ ...r, path: r.path.replace(/^\//, '') }) as RouteRecordRaw);

const router = createRouter({
  // Hash history: works under Tauri's custom protocol without server-side fallbacks.
  history: createWebHashHistory(),
  routes: [...blankRoutes, { path: '/', component: DefaultLayout, children: shellRoutes }],
  scrollBehavior: () => ({ top: 0 }),
});

router.beforeEach(async (to) => {
  // 21.03 §02-setup (Part 02 handoff §9, D-1/D-2): under the real Rust backend, the very first
  // decision is device role, not company data — an unconfigured device (no `device-settings.json`
  // yet) has no database to hold a company shell at all. `usesRust('setup')` is false in every
  // browser/e2e run (`isTauri()` gates it), so this branch never fires there and today's
  // mock-backend behavior below is unchanged.
  if (usesRust('setup')) {
    // Part 04 E-2 (3): re-read a cached "no users" whenever it would decide this navigation (a
    // non-public page or /login bounced to /welcome, or /welcome itself) — the wizard, a legacy
    // import and the demo import each create the first users after the cache was filled.
    const revalidateFresh = !to.meta.public || to.name === 'login' || to.name === 'welcome';
    const deviceState = await deviceStateForNavigation(revalidateFresh);
    if (!deviceState.configured) return to.name === 'device-setup' ? true : { name: 'device-setup' };
    // Part 04 E-2 (2): configured but the database is down — "no users" is unknown, not a fresh
    // install. Hold the navigation: `ServerFailureScreen` (App.vue) covers the app, and App.vue
    // reloads once the database is back so this guard decides again on real data.
    if (await mustHoldForDatabase(deviceState)) return false;
    // Part 04 E-2 (3): /welcome (legacy import, start company, demo) only makes sense on an empty
    // company — every one of its cards needs an empty database (00-import step 1, 02-setup §3.3).
    if (to.name === 'welcome' && deviceState.hasUsers) return { name: 'login' };
  }

  if (to.name === 'welcome') return true;
  if (isFreshInstall() && !to.meta.public) return { name: 'welcome' };
  // The login page itself is only reachable once a company exists (fresh or demo).
  if (isFreshInstall() && to.name === 'login') return { name: 'welcome' };

  const auth = useAuthStore();
  await auth.restore();

  if (to.meta.public) {
    if (to.name === 'login' && auth.isAuthenticated) return { name: 'home' };
    return true;
  }
  if (!auth.isAuthenticated) {
    return { name: 'login', query: to.fullPath !== '/' ? { redirect: to.fullPath } : undefined };
  }

  await useSettingsStore().load();

  if (to.meta.area && !auth.can(to.meta.area, to.meta.access ?? 'read')) {
    return { name: 'forbidden', query: { from: to.fullPath } };
  }
  return true;
});

router.afterEach((to) => {
  const store = useSettingsStore().settings?.storeName;
  document.title = [to.meta.title, store ?? APP_NAME_AR].filter(Boolean).join(' — ');
});

export default router;
