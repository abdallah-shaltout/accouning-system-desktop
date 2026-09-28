import type { RouteRecordRaw } from 'vue-router';

const section = 'الإعداد';

const routes: RouteRecordRaw[] = [
  {
    path: '/device-setup',
    name: 'device-setup',
    component: () => import('../pages/DeviceSetupPage.vue'),
    // Public + blank layout, same as /welcome (Part 02 handoff §9, D-2) — a first-run device has no
    // database connection yet, so this route must be reachable before any session exists. Dormant
    // under the mock backend: the router guard that redirects here only runs `usesRust('setup')`
    // (src/router/index.ts — needs a manager edit, see this wave's final report).
    meta: { public: true, layout: 'blank', title: 'إعداد الجهاز' },
  },
  {
    path: '/setup',
    name: 'setup-wizard',
    component: () => import('../pages/SetupWizardPage.vue'),
    // Public + blank layout, same as /welcome — a fresh install has no logged-in user yet, and the
    // wizard itself decides when the empty-company shell exists (see SetupWizardPage.vue).
    meta: { public: true, layout: 'blank', title: 'إعداد الشركة' },
  },
  {
    path: '/setup/opening',
    name: 'setup-opening',
    component: () => import('../pages/OpeningBalancesPage.vue'),
    meta: { title: 'الأرصدة الافتتاحية', section, area: 'accounting', access: 'write' },
  },
];

export default routes;
