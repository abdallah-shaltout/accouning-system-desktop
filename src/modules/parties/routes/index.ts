import type { RouteRecordRaw } from 'vue-router';

const section = 'العملاء والموردين';
const list = () => import('../pages/PartyListPage.vue');
const detail = () => import('../pages/PartyDetailPage.vue');
const form = () => import('../pages/PartyFormPage.vue');

const routes: RouteRecordRaw[] = [
  { path: '/customers', name: 'customers', component: list, props: { kind: 'customer' }, meta: { title: 'العملاء', section, area: 'parties' } },
  { path: '/customers/new', name: 'customer-new', component: form, props: { kind: 'customer' }, meta: { title: 'عميل جديد', section, area: 'parties', access: 'write' } },
  { path: '/customers/:id/edit', name: 'customer-edit', component: form, props: { kind: 'customer' }, meta: { title: 'تعديل عميل', section, area: 'parties', access: 'write' } },
  { path: '/customers/:id', name: 'customer', component: detail, props: { kind: 'customer' }, meta: { title: 'بطاقة عميل', section, area: 'parties' } },
  { path: '/suppliers', name: 'suppliers', component: list, props: { kind: 'supplier' }, meta: { title: 'الموردين', section, area: 'parties' } },
  { path: '/suppliers/new', name: 'supplier-new', component: form, props: { kind: 'supplier' }, meta: { title: 'مورد جديد', section, area: 'parties', access: 'write' } },
  { path: '/suppliers/:id/edit', name: 'supplier-edit', component: form, props: { kind: 'supplier' }, meta: { title: 'تعديل مورد', section, area: 'parties', access: 'write' } },
  { path: '/suppliers/:id', name: 'supplier', component: detail, props: { kind: 'supplier' }, meta: { title: 'بطاقة مورد', section, area: 'parties' } },
];

export default routes;
