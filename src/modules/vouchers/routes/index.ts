import type { RouteRecordRaw } from 'vue-router';

const section = 'المدفوعات';

const routes: RouteRecordRaw[] = [
  { path: '/vouchers', name: 'vouchers', component: () => import('../pages/VoucherListPage.vue'), meta: { title: 'السندات العامة', section, area: 'payments' } },
  { path: '/vouchers/new', name: 'voucher-new', component: () => import('../pages/VoucherFormPage.vue'), meta: { title: 'سند عام جديد', section, area: 'payments', access: 'write' } },
  { path: '/vouchers/:id', name: 'voucher-detail', component: () => import('../pages/VoucherDetailPage.vue'), meta: { title: 'تفاصيل السند', section, area: 'payments' } },
  { path: '/print/vouchers/:id', name: 'voucher-print', component: () => import('../pages/VoucherPrintPage.vue'), meta: { title: 'طباعة السند', layout: 'blank' } },
  // v2 phase 8 (docs/v2/09-purchases-payments-expenses.md §2 "Card settlement") — completes Phase
  // 3's TODO(phase 8) in src/mocks/backend/sales.ts.
  { path: '/payments/settlements', name: 'card-settlements', component: () => import('../pages/CardSettlementPage.vue'), meta: { title: 'تسوية البطاقات', section, area: 'payments', access: 'write' } },
];

export default routes;
