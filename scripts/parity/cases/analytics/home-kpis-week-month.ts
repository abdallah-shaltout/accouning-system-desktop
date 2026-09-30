/**
 * L5 (03-domains/14-analytics.md §8(b)): `getHomeKpis` for the `week` and `month` periods (on top
 * of `commands-defaults`'s `today` default) — exercises `periodRange`'s three branches and the
 * `changePct` 0/null/value cases across wider windows.
 */
import { defineCase } from '../../case';
import * as dashboardService from '../../../../src/modules/core/services/dashboardService';

export default defineCase({
  name: 'analytics/home-kpis-week-month',
  source: '03-domains/14-analytics.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('home-kpis-today', () => dashboardService.getHomeKpis('today'));
    await s.step('home-kpis-week', () => dashboardService.getHomeKpis('week'));
    await s.step('home-kpis-month', () => dashboardService.getHomeKpis('month'));
    await s.step('top-products-week', () => dashboardService.getTopProducts('week'));
    await s.step('top-products-month', () => dashboardService.getTopProducts('month'));
    await s.step('top-customers-week', () => dashboardService.getTopCustomers('week'));
    await s.step('top-customers-month', () => dashboardService.getTopCustomers('month'));
  },
});
