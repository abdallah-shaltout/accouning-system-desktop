/**
 * L5 (03-domains/14-analytics.md §8(b)): commands 1–10 with their default arguments — the 3
 * analytics tabs (sales/product/customer) plus the 7 async dashboard reads (summary, home KPIs,
 * low stock, recent invoices, recent activity, top products, top customers) over the seeded
 * demo-sa data with the clock pinned to the seed's "today" (2026-06-30).
 */
import { defineCase } from '../../case';
import * as analyticsService from '../../../../src/modules/analytics/services/analyticsService';
import * as dashboardService from '../../../../src/modules/core/services/dashboardService';

export default defineCase({
  name: 'analytics/commands-defaults',
  source: '03-domains/14-analytics.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('sales-analytics', () => analyticsService.getSalesAnalytics());
    await s.step('product-analytics', () => analyticsService.getProductAnalytics());
    await s.step('customer-analytics', () => analyticsService.getCustomerAnalytics());
    await s.step('dashboard-summary', () => dashboardService.getDashboardSummary());
    await s.step('home-kpis', () => dashboardService.getHomeKpis());
    await s.step('low-stock-products', () => dashboardService.getLowStockProducts());
    await s.step('recent-invoices', () => dashboardService.getRecentInvoices());
    await s.step('recent-activity', () => dashboardService.getRecentActivity());
    await s.step('top-products', () => dashboardService.getTopProducts());
    await s.step('top-customers', () => dashboardService.getTopCustomers());
  },
});
