import type { RouteRecordRaw } from 'vue-router';

const section = 'الحسابات';

const routes: RouteRecordRaw[] = [
  { path: '/accounting/accounts', name: 'accounts', component: () => import('../pages/ChartOfAccountsPage.vue'), meta: { title: 'دليل الحسابات', section, area: 'accounting' } },
  { path: '/accounting/journal', name: 'journal', component: () => import('../pages/JournalListPage.vue'), meta: { title: 'القيود اليومية', section, area: 'accounting' } },
  { path: '/accounting/journal/new', name: 'journal-new', component: () => import('../pages/JournalEntryFormPage.vue'), meta: { title: 'قيد يدوي', section, area: 'accounting', access: 'write' } },
  { path: '/accounting/journal/:id', name: 'journal-entry', component: () => import('../pages/JournalDetailPage.vue'), meta: { title: 'تفاصيل القيد', section, area: 'accounting' } },
  { path: '/accounting/journal-templates', name: 'journal-templates', component: () => import('../pages/JournalTemplatesPage.vue'), meta: { title: 'قوالب القيود والقيود المتكررة', section, area: 'accounting' } },
  { path: '/accounting/fiscal-years', name: 'fiscal-years', component: () => import('../pages/FiscalYearsPage.vue'), meta: { title: 'السنة المالية', section, area: 'accounting' } },
  { path: '/accounting/vat-settlement', name: 'vat-settlement', component: () => import('../pages/VatSettlementPage.vue'), meta: { title: 'تسوية ضريبة القيمة المضافة', section, area: 'accounting' } },
  // The day book prints from the reports hub's official layout; this old print route just forwards there with the preview open.
  { path: '/accounting/day-book', name: 'day-book', redirect: (to) => ({ path: '/reports/day-book', query: { ...to.query, print: '1' } }) },
];

export default routes;
