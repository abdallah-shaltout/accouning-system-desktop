import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';
import DefaultLayout from '@/modules/core/components/layout/DefaultLayout.vue';
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
