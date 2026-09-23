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
];

export default routes;
