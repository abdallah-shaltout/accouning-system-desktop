import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';
import DefaultLayout from '@/modules/core/components/layout/DefaultLayout.vue';
import { db } from '@/mocks/db';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
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

/**
 * No persisted IndexedDB snapshot AND an empty `db` means this is a genuinely fresh install —
 * `bootMockDb()` (awaited before the app mounts, see main.ts) left `db` empty on purpose so the
 * welcome screen can decide (demo data seeds it, "start company" creates an empty shell). Once
 * either card has run, `db.users` is non-empty, so this only ever fires once per install.
 */
function isFreshInstall(): boolean {
  return db.users.length === 0;
}

router.beforeEach(async (to) => {
  if (to.name === 'welcome') return true;
  if (isFreshInstall() && !to.meta.public) return { name: 'welcome' };
  // The login page itself is only reachable once a company exists (fresh or demo).
  if (isFreshInstall() && to.name === 'login') return { name: 'welcome' };

  const auth = useAuthStore();
  await auth.restore();

  if (to.meta.public) {
    if (to.name === 'login' && auth.isAuthenticated) return { path: '/' };
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
  document.title = [to.meta.title, store ?? 'نظام المحاسبة ونقاط البيع'].filter(Boolean).join(' — ');
});

export default router;
