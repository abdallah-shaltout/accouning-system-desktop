import type { RouteRecordRaw } from 'vue-router';

const section = 'المبيعات';

const routes: RouteRecordRaw[] = [
  { path: '/pos', name: 'pos', component: () => import('../pages/PosPage.vue'), meta: { title: 'نقطة البيع', area: 'pos', access: 'write', layout: 'blank' } },
  { path: '/pos/shifts', name: 'pos-shifts', component: () => import('../pages/ShiftsManagerPage.vue'), meta: { title: 'إدارة الورديات', section, area: 'pos' } },
  { path: '/pos/shifts/:id', name: 'pos-shift-report', component: () => import('../pages/ShiftReportPage.vue'), meta: { title: 'تقرير Z', section, area: 'pos' } },
  { path: '/print/invoices/:id', name: 'invoice-print', component: () => import('../pages/InvoicePrintPage.vue'), meta: { title: 'معاينة الطباعة', layout: 'blank' } },
  { path: '/invoices', name: 'invoices', component: () => import('../pages/InvoiceListPage.vue'), meta: { title: 'الفواتير', section, area: 'sales' } },
  { path: '/invoices/:id', name: 'invoice', component: () => import('../pages/InvoiceDetailPage.vue'), meta: { title: 'تفاصيل الفاتورة', section, area: 'sales' } },
  { path: '/invoices/:id/refund', name: 'invoice-refund', component: () => import('../pages/RefundPage.vue'), meta: { title: 'مرتجع مبيعات', section, area: 'sales', access: 'write' } },
  {
    path: '/sales/invoices/new',
    name: 'invoice-new',
    component: () => import('../pages/InvoiceFormPage.vue'),
    meta: { title: 'فاتورة مبيعات جديدة', section, area: 'sales', access: 'write' },
  },
  { path: '/sales/quotations', name: 'quotations', component: () => import('../pages/QuotationListPage.vue'), meta: { title: 'عروض الأسعار', section, area: 'sales' } },
  {
    path: '/sales/quotations/new',
    name: 'quotation-new',
    component: () => import('../pages/InvoiceFormPage.vue'),
    props: { asQuotation: true },
    meta: { title: 'عرض سعر جديد', section, area: 'sales', access: 'write' },
  },
  { path: '/sales/quotations/:id', name: 'quotation', component: () => import('../pages/QuotationDetailPage.vue'), meta: { title: 'تفاصيل عرض السعر', section, area: 'sales' } },
];

export default routes;
