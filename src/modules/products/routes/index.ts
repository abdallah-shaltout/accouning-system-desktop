import type { RouteRecordRaw } from 'vue-router';

const section = 'المنتجات والمخزون';

const routes: RouteRecordRaw[] = [
  { path: '/products', name: 'products', component: () => import('../pages/ProductListPage.vue'), meta: { title: 'المنتجات', section, area: 'inventory' } },
  { path: '/products/new', name: 'product-new', component: () => import('../pages/ProductFormPage.vue'), meta: { title: 'منتج جديد', section, area: 'inventory', access: 'write' } },
  { path: '/products/:id', name: 'product', component: () => import('../pages/ProductDetailPage.vue'), meta: { title: 'بطاقة منتج', section, area: 'inventory' } },
  { path: '/products/:id/edit', name: 'product-edit', component: () => import('../pages/ProductFormPage.vue'), meta: { title: 'تعديل منتج', section, area: 'inventory', access: 'write' } },
  { path: '/catalog/categories', name: 'categories', component: () => import('../pages/CategoriesUnitsPage.vue'), meta: { title: 'التصنيفات والوحدات', section, area: 'inventory' } },
  { path: '/catalog/price-lists', name: 'price-lists', component: () => import('../pages/PriceListsPage.vue'), meta: { title: 'قوائم الأسعار', section, area: 'inventory' } },
  { path: '/inventory/adjustments', name: 'adjustments', component: () => import('../pages/StockAdjustmentListPage.vue'), meta: { title: 'تسويات المخزون', section, area: 'inventory' } },
  { path: '/inventory/adjustments/new', name: 'adjustment-new', component: () => import('../pages/StockAdjustmentFormPage.vue'), meta: { title: 'تسوية جديدة', section, area: 'inventory', access: 'write' } },
  { path: '/inventory/adjustments/:id', name: 'adjustment', component: () => import('../pages/StockAdjustmentDetailPage.vue'), meta: { title: 'تفاصيل التسوية', section, area: 'inventory' } },
  { path: '/inventory/movements', name: 'movements', component: () => import('../pages/StockMovementsPage.vue'), meta: { title: 'حركة المخزون', section, area: 'inventory' } },
  // v2 phase 6 §5 — Stocktake v2 (scope, snapshot, blind count, scan counting, review).
  { path: '/inventory/counts', name: 'counts', component: () => import('../pages/StockCountListPage.vue'), meta: { title: 'الجرد', section, area: 'inventory' } },
  { path: '/inventory/counts/new', name: 'count-new', component: () => import('../pages/StockCountNewPage.vue'), meta: { title: 'جرد جديد', section, area: 'inventory', access: 'write' } },
  { path: '/inventory/counts/:id', name: 'count', component: () => import('../pages/StockCountDetailPage.vue'), meta: { title: 'تفاصيل الجرد', section, area: 'inventory' } },
  // v2 phase 6 §4 — Expiry report (expired/≤30/≤60/≤90, grouped by supplier).
  { path: '/inventory/expiry', name: 'expiry', component: () => import('../pages/ExpiryReportPage.vue'), meta: { title: 'تقرير الصلاحية', section, area: 'inventory' } },
  // v2 phase 9 (docs/v2/07-products-and-inventory.md §4, deferred from phase 6): branch transfers.
  { path: '/inventory/transfers', name: 'transfers', component: () => import('../pages/StockTransferListPage.vue'), meta: { title: 'تحويلات الفروع', section, area: 'inventory' } },
];

export default routes;
