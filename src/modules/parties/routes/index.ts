import type { RouteRecordRaw } from 'vue-router';

const section = 'العملاء والموردين';
const list = () => import('../pages/PartyListPage.vue');
const detail = () => import('../pages/PartyDetailPage.vue');

const routes: RouteRecordRaw[] = [
  { path: '/customers', name: 'customers', component: list, props: { kind: 'customer' }, meta: { title: 'العملاء', section, area: 'parties' } },
  { path: '/customers/:id', name: 'customer', component: detail, props: { kind: 'customer' }, meta: { title: 'بطاقة عميل', section, area: 'parties' } },
  { path: '/suppliers', name: 'suppliers', component: list, props: { kind: 'supplier' }, meta: { title: 'الموردين', section, area: 'parties' } },
  { path: '/suppliers/:id', name: 'supplier', component: detail, props: { kind: 'supplier' }, meta: { title: 'بطاقة مورد', section, area: 'parties' } },
];

export default routes;
