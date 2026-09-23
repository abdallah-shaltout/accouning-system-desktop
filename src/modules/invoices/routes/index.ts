import type { RouteRecordRaw } from 'vue-router';

const section = 'المبيعات';

const routes: RouteRecordRaw[] = [
  { path: '/pos', name: 'pos', component: () => import('../pages/PosPage.vue'), meta: { title: 'نقطة البيع', area: 'pos', access: 'write', layout: 'blank' } },
  { path: '/print/invoices/:id', name: 'invoice-print', component: () => import('../pages/InvoicePrintPage.vue'), meta: { title: 'معاينة الطباعة', layout: 'blank' } },
  { path: '/invoices', name: 'invoices', component: () => import('../pages/InvoiceListPage.vue'), meta: { title: 'الفواتير', section, area: 'sales' } },
  { path: '/invoices/:id', name: 'invoice', component: () => import('../pages/InvoiceDetailPage.vue'), meta: { title: 'تفاصيل الفاتورة', section, area: 'sales' } },
  { path: '/invoices/:id/refund', name: 'invoice-refund', component: () => import('../pages/RefundPage.vue'), meta: { title: 'مرتجع مبيعات', section, area: 'sales', access: 'write' } },
];

export default routes;
