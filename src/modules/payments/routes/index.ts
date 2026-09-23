import type { RouteRecordRaw } from 'vue-router';

const section = 'المدفوعات';

const routes: RouteRecordRaw[] = [
  { path: '/payments', name: 'payments', component: () => import('../pages/PaymentListPage.vue'), meta: { title: 'سندات القبض والصرف', section, area: 'payments' } },
  { path: '/payments/new', name: 'payment-new', component: () => import('../pages/PaymentFormPage.vue'), meta: { title: 'سند جديد', section, area: 'payments', access: 'write' } },
  { path: '/payments/:id', name: 'payment-detail', component: () => import('../pages/PaymentDetailPage.vue'), meta: { title: 'تفاصيل السند', section, area: 'payments' } },
];

export default routes;
