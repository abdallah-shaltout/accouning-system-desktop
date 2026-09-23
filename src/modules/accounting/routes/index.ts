import type { RouteRecordRaw } from 'vue-router';

const section = 'الحسابات';

const routes: RouteRecordRaw[] = [
  { path: '/accounting/accounts', name: 'accounts', component: () => import('../pages/ChartOfAccountsPage.vue'), meta: { title: 'دليل الحسابات', section, area: 'accounting' } },
  { path: '/accounting/journal', name: 'journal', component: () => import('../pages/JournalListPage.vue'), meta: { title: 'القيود اليومية', section, area: 'accounting' } },
  { path: '/accounting/journal/new', name: 'journal-new', component: () => import('../pages/JournalEntryFormPage.vue'), meta: { title: 'قيد يدوي', section, area: 'accounting', access: 'write' } },
  { path: '/accounting/journal/:id', name: 'journal-entry', component: () => import('../pages/JournalDetailPage.vue'), meta: { title: 'تفاصيل القيد', section, area: 'accounting' } },
  { path: '/accounting/fiscal-years', name: 'fiscal-years', component: () => import('../pages/FiscalYearsPage.vue'), meta: { title: 'السنة المالية', section, area: 'accounting' } },
];

export default routes;
