import type { RouteRecordRaw } from 'vue-router';

const section = 'التقارير';
const meta = (title: string) => ({ title, section, area: 'reports' as const });

const routes: RouteRecordRaw[] = [
  { path: '/reports', name: 'reports', component: () => import('../pages/ReportsHubPage.vue'), meta: { title: 'التقارير', area: 'reports' } },

  // --- Financial statements ---------------------------------------------------------------------
  { path: '/reports/trial-balance', name: 'report-trial-balance', component: () => import('../pages/TrialBalancePage.vue'), meta: meta('ميزان المراجعة') },
  { path: '/reports/profit-loss', name: 'report-profit-loss', component: () => import('../pages/ProfitLossPage.vue'), meta: meta('قائمة الدخل') },
  { path: '/reports/balance-sheet', name: 'report-balance-sheet', component: () => import('../pages/BalanceSheetPage.vue'), meta: meta('الميزانية العمومية') },
  { path: '/reports/cash-flow', name: 'report-cash-flow', component: () => import('../pages/CashFlowPage.vue'), meta: meta('قائمة التدفقات النقدية') },
  { path: '/reports/ledger', name: 'report-ledger', component: () => import('../pages/LedgerPage.vue'), meta: meta('كشف حساب') },
  { path: '/reports/day-book', name: 'report-day-book', component: () => import('../pages/DayBookPage.vue'), meta: meta('دفتر اليومية') },
  // v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §3): P&L by cost center.
  { path: '/reports/cost-centers', name: 'report-cost-centers', component: () => import('../pages/CostCenterPnlPage.vue'), meta: meta('الأرباح حسب مركز التكلفة') },

  // --- Receivables & payables ----------------------------------------------------------------
  { path: '/reports/aging', name: 'report-aging', component: () => import('../pages/AgingReportPage.vue'), meta: meta('أعمار الديون') },
  { path: '/reports/overdue', name: 'report-overdue', component: () => import('../pages/OverdueReportPage.vue'), meta: meta('المستندات المتأخرة') },

  // --- Sales -----------------------------------------------------------------------------------
  { path: '/reports/sales', name: 'report-sales', component: () => import('../pages/SalesReportPage.vue'), meta: meta('تقرير المبيعات') },
  { path: '/reports/gross-profit', name: 'report-gross-profit', component: () => import('../pages/GrossProfitPage.vue'), meta: meta('تقرير مجمل الربح') },
  { path: '/reports/returns', name: 'report-returns', component: () => import('../pages/ReturnsReportPage.vue'), meta: meta('تحليل المرتجعات') },
  { path: '/reports/discounts', name: 'report-discounts', component: () => import('../pages/DiscountsReportPage.vue'), meta: meta('الخصومات وتجاوزات السعر') },
  { path: '/reports/shifts', name: 'report-shifts', component: () => import('../pages/ShiftsReportPage.vue'), meta: meta('سجل الورديات') },

  // --- Inventory ------------------------------------------------------------------------------
  { path: '/reports/inventory', name: 'report-inventory', component: () => import('../pages/InventoryReportPage.vue'), meta: meta('تقرير المخزون') },
  { path: '/reports/stock-health', name: 'report-stock-health', component: () => import('../pages/StockHealthReportPage.vue'), meta: meta('المخزون المنخفض والراكد') },
  { path: '/reports/stocktake-variances', name: 'report-stocktake-variances', component: () => import('../pages/StocktakeVariancesPage.vue'), meta: meta('فروقات الجرد') },
  { path: '/reports/transfers', name: 'report-transfers', component: () => import('../pages/TransfersReportPage.vue'), meta: meta('تقرير التحويلات') },

  // --- Purchases & expenses ---------------------------------------------------------------------
  { path: '/reports/purchases', name: 'report-purchases', component: () => import('../pages/PurchasesReportPage.vue'), meta: meta('تقرير المشتريات') },
  { path: '/reports/expenses', name: 'report-expenses', component: () => import('../pages/ExpensesReportPage.vue'), meta: meta('تقرير المصروفات') },
  { path: '/reports/budget-vs-actual', name: 'report-budget-vs-actual', component: () => import('../pages/BudgetVsActualPage.vue'), meta: meta('الميزانية مقابل الفعلي') },

  // --- Tax -------------------------------------------------------------------------------------
  { path: '/reports/vat', name: 'report-vat', component: () => import('../pages/VatReportPage.vue'), meta: meta('ملخص الضريبة') },
  { path: '/reports/vat-detail', name: 'report-vat-detail', component: () => import('../pages/VatDetailPage.vue'), meta: meta('التفصيل الضريبي') },

  // --- Management ------------------------------------------------------------------------------
  { path: '/reports/period-comparison', name: 'report-period-comparison', component: () => import('../pages/PeriodComparisonPage.vue'), meta: meta('مقارنة الفترات') },
  { path: '/reports/branch-comparison', name: 'report-branch-comparison', component: () => import('../pages/BranchComparisonPage.vue'), meta: meta('مقارنة الفروع') },
  { path: '/reports/business-health', name: 'report-business-health', component: () => import('../pages/BusinessHealthPage.vue'), meta: meta('الصحة المالية') },
  { path: '/reports/profit-leakage', name: 'report-profit-leakage', component: () => import('../pages/ProfitLeakagePage.vue'), meta: meta('تسرب الربح') },
];

export default routes;
