import type { RouteRecordRaw } from 'vue-router';

/** v2 phase 10 (docs/v2/11-journal-dashboard-insights.md Part C). */
const routes: RouteRecordRaw[] = [
  { path: '/analytics', name: 'analytics', component: () => import('./pages/AnalyticsPage.vue'), meta: { title: 'التحليلات', area: 'analytics' } },
];

export default routes;
