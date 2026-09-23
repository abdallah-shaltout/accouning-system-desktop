import type { RouteRecordRaw } from 'vue-router';

const section = 'المصروفات';

const routes: RouteRecordRaw[] = [
  { path: '/expenses', name: 'expenses', component: () => import('../pages/ExpenseListPage.vue'), meta: { title: 'المصروفات', section, area: 'expenses' } },
  { path: '/expenses/new', name: 'expense-new', component: () => import('../pages/ExpenseFormPage.vue'), meta: { title: 'مصروف جديد', section, area: 'expenses', access: 'write' } },
  { path: '/expenses/recurring', name: 'expenses-recurring', component: () => import('../pages/RecurringExpensesPage.vue'), meta: { title: 'المصروفات المتكررة', section, area: 'expenses', access: 'write' } },
  { path: '/expenses/:id', name: 'expense-detail', component: () => import('../pages/ExpenseDetailPage.vue'), meta: { title: 'تفاصيل المصروف', section, area: 'expenses' } },
  {
    path: '/settings/expenses',
    name: 'settings-expenses',
    component: () => import('../pages/ExpenseCategoriesSettingsPage.vue'),
    meta: { title: 'تصنيفات المصروفات', section: 'الإعدادات', area: 'settings' },
  },
];

export default routes;
