import type { RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  {
    path: '/approvals',
    name: 'approvals',
    component: () => import('../pages/ApprovalsPage.vue'),
    meta: { title: 'طلبات الاعتماد', section: 'الإدارة', area: 'approvals' },
  },
];

export default routes;
