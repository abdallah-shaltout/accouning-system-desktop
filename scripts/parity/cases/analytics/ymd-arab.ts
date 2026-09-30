/**
 * L5 (03-domains/14-analytics.md §8(b)): command 1 (`getSalesAnalytics`) with the `ymd` date style
 * and `arab` numerals (decision A-2: display prefs travel as command args on the Rust side; here
 * they drive the mock's own `formatDate`/`numeralSystem` reactive refs so `trendInsight`'s embedded
 * date/number strings are produced under the same prefs both backends will see).
 */
import { defineCase } from '../../case';
import * as analyticsService from '../../../../src/modules/analytics/services/analyticsService';
import { setDateFormatStyle } from '../../../../src/modules/core/controllers/useAppearance';
import { setNumerals } from '../../../../src/modules/core/helpers/format';

export default defineCase({
  name: 'analytics/ymd-arab',
  source: '03-domains/14-analytics.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    setDateFormatStyle('ymd');
    setNumerals('arab');
    await s.step('sales-analytics-ymd-arab', () => analyticsService.getSalesAnalytics());
  },
});
