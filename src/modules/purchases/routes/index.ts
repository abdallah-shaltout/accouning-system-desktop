import type { RouteRecordRaw } from 'vue-router';

const section = 'المشتريات';

const routes: RouteRecordRaw[] = [
  { path: '/purchases', name: 'purchases', component: () => import('../pages/PurchaseListPage.vue'), meta: { title: 'أوامر الشراء', section, area: 'purchases' } },
  { path: '/purchases/new', name: 'purchase-new', component: () => import('../pages/PurchaseFormPage.vue'), meta: { title: 'أمر شراء جديد', section, area: 'purchases', access: 'write' } },
  { path: '/purchases/:id', name: 'purchase', component: () => import('../pages/PurchaseDetailPage.vue'), meta: { title: 'تفاصيل أمر الشراء', section, area: 'purchases' } },
  { path: '/purchases/:id/edit', name: 'purchase-edit', component: () => import('../pages/PurchaseFormPage.vue'), meta: { title: 'تعديل أمر شراء', section, area: 'purchases', access: 'write' } },
  { path: '/purchases/:id/receive', name: 'purchase-receive', component: () => import('../pages/PurchaseReceivePage.vue'), meta: { title: 'استلام أمر شراء', section, area: 'purchases', access: 'write' } },
  { path: '/purchases/:id/return', name: 'purchase-return', component: () => import('../pages/PurchaseReturnPage.vue'), meta: { title: 'مرتجع مشتريات', section, area: 'purchases', access: 'write' } },
  { path: '/print/purchases/:id', name: 'purchase-print', component: () => import('../pages/PurchasePrintPage.vue'), meta: { title: 'طباعة أمر الشراء', layout: 'blank' } },
];

export default routes;
