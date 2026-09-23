import type { RouteRecordRaw } from 'vue-router';

const section = 'التقارير';
const meta = (title: string) => ({ title, section, area: 'reports' as const });

const routes: RouteRecordRaw[] = [
  { path: '/reports', name: 'reports', component: () => import('../pages/ReportsHubPage.vue'), meta: { title: 'التقارير', area: 'reports' } },
  { path: '/reports/trial-balance', name: 'report-trial-balance', component: () => import('../pages/TrialBalancePage.vue'), meta: meta('ميزان المراجعة') },
  { path: '/reports/profit-loss', name: 'report-profit-loss', component: () => import('../pages/ProfitLossPage.vue'), meta: meta('قائمة الدخل') },
  { path: '/reports/balance-sheet', name: 'report-balance-sheet', component: () => import('../pages/BalanceSheetPage.vue'), meta: meta('الميزانية العمومية') },
  { path: '/reports/ledger', name: 'report-ledger', component: () => import('../pages/LedgerPage.vue'), meta: meta('كشف حساب') },
  { path: '/reports/sales', name: 'report-sales', component: () => import('../pages/SalesReportPage.vue'), meta: meta('تقرير المبيعات') },
  { path: '/reports/inventory', name: 'report-inventory', component: () => import('../pages/InventoryReportPage.vue'), meta: meta('تقرير المخزون') },
  { path: '/reports/vat', name: 'report-vat', component: () => import('../pages/VatReportPage.vue'), meta: meta('ملخص الضريبة') },
  // v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §3): P&L by cost center.
  { path: '/reports/cost-centers', name: 'report-cost-centers', component: () => import('../pages/CostCenterPnlPage.vue'), meta: meta('الأرباح حسب مركز التكلفة') },
];

export default routes;
