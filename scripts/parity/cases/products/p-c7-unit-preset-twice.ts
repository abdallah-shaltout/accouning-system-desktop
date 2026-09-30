/**
 * 06-products §8(b) P-C7: a unit preset creates only the names that don't exist yet (exact match),
 * in list order, and returns only those; applying it again creates nothing.
 */
import { defineCase } from '../../case';
import * as catalogService from '../../../../src/modules/products/services/catalogService';

export default defineCase({
  name: 'products/p-c7-unit-preset-twice',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.step('pharmacy-1', () => catalogService.applyUnitPreset('pharmacy'));
    await s.step('pharmacy-2', () => catalogService.applyUnitPreset('pharmacy'));
    await s.step('clothing-1', () => catalogService.applyUnitPreset('clothing'));
    await s.step('supermarket-1', () => catalogService.applyUnitPreset('supermarket'));
    await s.step('supermarket-2', () => catalogService.applyUnitPreset('supermarket'));
    await s.step('units', () => catalogService.getUnits());
  },
});
