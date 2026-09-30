/**
 * 12b-period-close §8(b): fiscal years (§3.1) — list order (start date desc), the current year
 * (today inside a year, else the latest), create, update, and the `saveFiscalYear` refusals in the
 * mock's order: name, end ≤ start, overlap (CONFLICT naming the other year), unknown id.
 *
 * Editing a closed year: Rust refuses it (FORBIDDEN, decision P-D1), the mock saves it. The step
 * records the outcome instead of aborting, sends the year unchanged (so the mock's save changes
 * nothing), and is allowlisted citing P-D1.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import { toRecordedError } from '../../books';

export default defineCase({
  name: 'accounting/fiscal-year-crud',
  source: '03-domains/12b-period-close.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  allow: [{ path: 'steps.edit-closed-year.value', reason: '12b-period-close P-D1: Rust refuses editing a closed fiscal year (FORBIDDEN); the mock saves it (the page never offers it)' }],
  async run(s) {
    await s.step('list', () => accountingService.getFiscalYears());
    await s.step('current', () => accountingService.getCurrentFiscalYear());
    const next = await s.step('create-2027', () => accountingService.saveFiscalYear({ name: '2027', startDate: '2027-01-01', endDate: '2027-12-31', isClosed: false }));
    await s.step('update-2027', () => accountingService.saveFiscalYear({ name: 'السنة المالية 2027', startDate: '2027-01-01', endDate: '2027-12-30', isClosed: false }, next.id));
    // Server-owned fields in the input (P-D3) are not sent by the page; the list shows the result.
    await s.step('list-after', () => accountingService.getFiscalYears());

    await s.expectError('name-blank', () => accountingService.saveFiscalYear({ name: '  ', startDate: '2028-01-01', endDate: '2028-12-31', isClosed: false }));
    await s.expectError('end-before-start', () => accountingService.saveFiscalYear({ name: '2028', startDate: '2028-12-31', endDate: '2028-01-01', isClosed: false }));
    await s.expectError('end-equals-start', () => accountingService.saveFiscalYear({ name: '2028', startDate: '2028-01-01', endDate: '2028-01-01', isClosed: false }));
    await s.expectError('dates-missing', () => accountingService.saveFiscalYear({ name: '2028', startDate: '', endDate: '2028-12-31', isClosed: false }));
    await s.expectError('overlap-2026', () => accountingService.saveFiscalYear({ name: 'متداخلة', startDate: '2026-07-01', endDate: '2027-06-30', isClosed: false }));
    await s.expectError('overlap-on-update', () => accountingService.saveFiscalYear({ name: 'السنة المالية 2027', startDate: '2026-12-31', endDate: '2027-12-30', isClosed: false }, next.id));
    await s.expectError('update-unknown', () => accountingService.saveFiscalYear({ name: '2030', startDate: '2030-01-01', endDate: '2030-12-31', isClosed: false }, 'fy-missing'));

    await s.step('edit-closed-year', async () => {
      try {
        return { saved: await accountingService.saveFiscalYear({ name: '2025', startDate: '2025-01-01', endDate: '2025-12-31', isClosed: true }, s.baseId('fy-2025')) };
      } catch (e) {
        return { refused: toRecordedError(e) };
      }
    });
    await s.setClock('2028-03-01T09:00:00.000Z');
    await s.step('current-after-all-years', () => accountingService.getCurrentFiscalYear());
  },
});
