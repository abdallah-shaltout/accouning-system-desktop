import type { RouteRecordRaw } from 'vue-router';

const section = 'الإعدادات';

const routes: RouteRecordRaw[] = [
  { path: '/settings', redirect: '/settings/general' },
  { path: '/settings/general', name: 'settings-general', component: () => import('../pages/GeneralSettingsPage.vue'), meta: { title: 'عام', section, area: 'settings' } },
  // Phase 3 (docs/v2/06-sales-and-pos.md §3, docs/v2/09-purchases-payments-expenses.md §2): tax
  // categories (S/Z/E/O) and payment methods, split out of the old "General & taxes" page.
  { path: '/settings/taxes', name: 'settings-taxes', component: () => import('../pages/TaxesSettingsPage.vue'), meta: { title: 'الضرائب', section, area: 'settings' } },
  { path: '/settings/payment-methods', name: 'settings-payment-methods', component: () => import('../pages/PaymentMethodsSettingsPage.vue'), meta: { title: 'طرق الدفع', section, area: 'settings' } },
  { path: '/settings/printing', name: 'settings-printing', component: () => import('../pages/PrintingSettingsPage.vue'), meta: { title: 'الطباعة', section, area: 'settings' } },
  // Appearance is a per-device preference, open to every signed-in user.
  { path: '/settings/appearance', name: 'settings-appearance', component: () => import('../pages/AppearanceSettingsPage.vue'), meta: { title: 'المظهر', section } },
  // Phase 13a: backup & restore (docs/v2/14-platform.md §4).
  { path: '/settings/backup', name: 'settings-backup', component: () => import('../pages/BackupSettingsPage.vue'), meta: { title: 'النسخ الاحتياطي', section, area: 'settings' } },
  // Phase 11a: PDF template designer (docs/v2/12-documents-pdf-excel.md §3). Pages live in their
  // own `modules/templates/` module, not `settings/pages/`, to avoid the concurrent backup work.
  { path: '/settings/templates', name: 'settings-templates', component: () => import('@/modules/templates/pages/TemplateListPage.vue'), meta: { title: 'قوالب الطباعة', section, area: 'settings' } },
  { path: '/settings/templates/:id', name: 'settings-template-designer', component: () => import('@/modules/templates/pages/TemplateDesignerPage.vue'), meta: { title: 'قالب الطباعة', section, area: 'settings' } },
];

export default routes;
