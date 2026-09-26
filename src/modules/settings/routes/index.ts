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
  // v2 phase 6 §7: custom product fields + unit presets.
  { path: '/settings/products', name: 'settings-products', component: () => import('../pages/ProductsSettingsPage.vue'), meta: { title: 'المنتجات', section, area: 'inventory', access: 'write' } },
  // v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md): branches, cost centers, currencies.
  { path: '/settings/branches', name: 'settings-branches', component: () => import('../pages/BranchesSettingsPage.vue'), meta: { title: 'الفروع', section, area: 'settings' } },
  { path: '/settings/cost-centers', name: 'settings-cost-centers', component: () => import('../pages/CostCentersSettingsPage.vue'), meta: { title: 'مراكز التكلفة', section, area: 'settings' } },
  { path: '/settings/currencies', name: 'settings-currencies', component: () => import('../pages/CurrenciesSettingsPage.vue'), meta: { title: 'العملات', section, area: 'settings' } },
  // v2 phase 10 (docs/v2/11-journal-dashboard-insights.md D1 "Thresholds"): insight-engine rule thresholds.
  { path: '/settings/recommendations', name: 'settings-recommendations', component: () => import('../pages/RecommendationsSettingsPage.vue'), meta: { title: 'التوصيات', section, area: 'settings' } },
  // v2 phase 6 §6: role matrix editor. Gated on `users` (only admins manage users/roles today).
  { path: '/settings/roles', name: 'settings-roles', component: () => import('../pages/RoleMatrixSettingsPage.vue'), meta: { title: 'المستخدمون والأدوار', section, area: 'users' } },
  // 18.B4: business audit trail (structured `db.audit`, DataTable + filters + diff view). Admin only.
  { path: '/settings/audit-log', name: 'settings-audit-log', component: () => import('../pages/AuditLogSettingsPage.vue'), meta: { title: 'سجل التدقيق', section, area: 'users' } },
  // Appearance is a per-device preference, open to every signed-in user.
  { path: '/settings/appearance', name: 'settings-appearance', component: () => import('../pages/AppearanceSettingsPage.vue'), meta: { title: 'المظهر', section } },
  // Keyboard shortcuts are also a per-device preference — same access as appearance.
  { path: '/settings/keyboard-shortcuts', name: 'settings-keyboard-shortcuts', component: () => import('../pages/KeyboardShortcutsSettingsPage.vue'), meta: { title: 'اختصارات لوحة المفاتيح', section } },
  // Phase 13a: backup & restore (docs/v2/14-platform.md §4).
  { path: '/settings/backup', name: 'settings-backup', component: () => import('../pages/BackupSettingsPage.vue'), meta: { title: 'النسخ الاحتياطي', section, area: 'settings' } },
  // Phase 11a: PDF template designer (docs/v2/12-documents-pdf-excel.md §3). Pages live in their
  // own `modules/templates/` module, not `settings/pages/`, to avoid the concurrent backup work.
  { path: '/settings/templates', name: 'settings-templates', component: () => import('@/modules/templates/pages/TemplateListPage.vue'), meta: { title: 'قوالب الطباعة', section, area: 'settings' } },
  { path: '/settings/templates/:id', name: 'settings-template-designer', component: () => import('@/modules/templates/pages/TemplateDesignerPage.vue'), meta: { title: 'قالب الطباعة', section, area: 'settings' } },
  // 18.B6: version info + "تصدير ملف التشخيص" support bundle. Open to every signed-in user — a
  // cashier hitting an error needs to be able to export the bundle without an admin around.
  { path: '/settings/about', name: 'settings-about', component: () => import('../pages/AboutSettingsPage.vue'), meta: { title: 'حول / الدعم', section } },
];

export default routes;
