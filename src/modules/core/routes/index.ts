import type { RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'home',
    component: () => import('../pages/DashboardPage.vue'),
    meta: { title: 'الرئيسية', area: 'dashboard' },
  },
  {
    path: '/forbidden',
    name: 'forbidden',
    component: () => import('../pages/ForbiddenPage.vue'),
    meta: { title: 'غير مصرح' },
  },
  // v2 phase C (docs/v2/16-equal-rebrand-and-ui-kit.md): dev-only shadcn-vue component gallery.
  // Not linked from anywhere; reachable only by typing the URL, and only in dev builds.
  ...(import.meta.env.DEV
    ? [
        { path: '/dev/ui', name: 'dev-ui', component: () => import('../pages/DevUiPage.vue'), meta: { title: 'معرض المكوّنات' } } satisfies RouteRecordRaw,
        // 18.B5: dev-only diagnostics inspector (errors/perf/debug/audit/accounting tabs).
        { path: '/dev/diagnostics', name: 'dev-diagnostics', component: () => import('@/modules/diagnostics/pages/DevDiagnosticsPage.vue'), meta: { title: 'التشخيص' } } satisfies RouteRecordRaw,
      ]
    : []),
  {
    path: '/:pathMatch(.*)*',
    name: 'not-found',
    component: () => import('../pages/NotFoundPage.vue'),
    meta: { title: 'غير موجود' },
  },
];

export default routes;
