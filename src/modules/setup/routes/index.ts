import type { RouteRecordRaw } from 'vue-router';

const section = 'الإعداد';

const routes: RouteRecordRaw[] = [
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
