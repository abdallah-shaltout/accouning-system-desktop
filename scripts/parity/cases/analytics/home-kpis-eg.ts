/**
 * L5 (phase-b2-parity-cases.md "L5" header: "plus one demo-eg pass each for the statements and
 * home KPIs"). `getHomeKpis`/`getDashboardSummary` against the `demo-eg` base (14% VAT, EGP,
 * Africa/Cairo timezone) — the one non-SA pass this lane's header calls for on the dashboard side.
 */
import { defineCase } from '../../case';
import * as dashboardService from '../../../../src/modules/core/services/dashboardService';

export default defineCase({
  name: 'analytics/home-kpis-eg',
  source: '03-domains/14-analytics.md §8(b); phase-b2-parity-cases.md L5 header',
  base: 'demo-eg',
  user: 'admin',
  async run(s) {
    await s.step('dashboard-summary', () => dashboardService.getDashboardSummary());
    await s.step('home-kpis-today', () => dashboardService.getHomeKpis('today'));
    await s.step('home-kpis-week', () => dashboardService.getHomeKpis('week'));
    await s.step('home-kpis-month', () => dashboardService.getHomeKpis('month'));
  },
});
