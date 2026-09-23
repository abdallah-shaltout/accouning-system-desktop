import type { RouteRecordRaw } from 'vue-router';

const section = 'الإعدادات';

const routes: RouteRecordRaw[] = [
  { path: '/settings', redirect: '/settings/general' },
  { path: '/settings/general', name: 'settings-general', component: () => import('../pages/GeneralSettingsPage.vue'), meta: { title: 'عام والضرائب', section, area: 'settings' } },
  { path: '/settings/printing', name: 'settings-printing', component: () => import('../pages/PrintingSettingsPage.vue'), meta: { title: 'الطباعة', section, area: 'settings' } },
  // Appearance is a per-device preference, open to every signed-in user.
  { path: '/settings/appearance', name: 'settings-appearance', component: () => import('../pages/AppearanceSettingsPage.vue'), meta: { title: 'المظهر', section } },
];

export default routes;
