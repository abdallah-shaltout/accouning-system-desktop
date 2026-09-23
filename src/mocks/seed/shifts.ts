/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §5 "Shifts"). Seeds a handful of realistic closed shifts
 * — one per of the last few demo days — retroactively tagging that day's cash POS sales/refunds as
 * belonging to the shift (so the X/Z-report "sales by method" and the close screen's expected-cash
 * math have real numbers), then closes each shift with a small, deliberate variance so
 * `verify:mocks` invariant 11 ("every closed shift: expected cash − counted cash = the posted
 * over/short amount") has real rows to check, not just the `todo` fallback.
 *
 * Runs AFTER `seedHistory()` (needs `db.invoices` already populated) — wired from `seed/index.ts`.
 * Doesn't touch `seed/history.ts`'s dense day-by-day replay at all: it works backward from the
 * invoices/refunds that replay already produced.
 */
import type { Shift, ShiftMovement } from '@/modules/invoices/types';
import { db, nextNumber } from '../db';
import { closeShift } from '../backend/shifts';
import { round2, sum, uid } from '../utils';

const TERMINAL = 'pos-1';
const CASHIER = 'usr-4';

/** Seeds `SHIFT_DAYS` closed shifts on the most recent days that have at least one cash POS sale. */
export function seedShifts(now = new Date(), shiftDays = 4): void {
  const cashMethodIds = new Set(db.paymentMethods.filter((m) => m.accountRole === 'cash').map((m) => m.id));

  // Group already-seeded invoices by local day (skip "today" — that shift is left open on purpose so
  // the POS/shift-bar demo has something live to show and close interactively).
  const todayKey = now.toDateString();
  const byDay = new Map<string, typeof db.invoices>();
  for (const inv of db.invoices) {
    const key = new Date(inv.date).toDateString();
    if (key === todayKey) continue;
    if (!(inv.tenders ?? []).some((t) => cashMethodIds.has(t.paymentMethodId))) continue;
    (byDay.get(key) ?? byDay.set(key, []).get(key)!).push(inv);
  }
  const days = [...byDay.keys()]
    .sort((a, b) => new Date(b).getTime() - new Date(a).getTime())
    .slice(0, shiftDays);

  let seq = 0;
  for (const dayKey of days.reverse()) {
    const dayInvoices = (byDay.get(dayKey) ?? []).sort((a, b) => a.date.localeCompare(b.date));
    if (!dayInvoices.length) continue;
    const firstTime = new Date(dayInvoices[0].date);
    const openAt = new Date(firstTime);
    openAt.setMinutes(openAt.getMinutes() - 15);
    const lastTime = new Date(dayInvoices.at(-1)!.date);
    const closeAt = new Date(lastTime);
    closeAt.setMinutes(closeAt.getMinutes() + 20);

    const openingFloat = 500;
    const shift: Shift = {
      id: uid('shift'),
      number: nextNumber('shift'),
      terminalId: TERMINAL,
      status: 'OPEN',
      openedBy: CASHIER,
      openedAt: openAt.toISOString(),
      openingFloat,
      openingDenominations: [
        { value: 100, count: 3 },
        { value: 50, count: 4 },
        { value: 10, count: 0 },
      ],
      movements: [],
    };
    db.shifts.push(shift);

    // Tag this day's cash invoices as belonging to the shift and log each as a SALE_CASH movement —
    // mirrors what `sales.ts`'s `recordSale` does live when a shift is open at checkout time.
    const movements: ShiftMovement[] = [];
    for (const inv of dayInvoices) {
      inv.shiftId = shift.id;
      inv.source = 'POS';
      const cashAmount = round2(sum((inv.tenders ?? []).filter((t) => cashMethodIds.has(t.paymentMethodId)), (t) => t.amount));
      if (cashAmount > 0) {
        movements.push({ id: uid('shmv'), kind: 'SALE_CASH', amount: cashAmount, refId: inv.id, refNumber: inv.number, at: inv.date, by: CASHIER });
      }
    }
    // A small pay-out (e.g. a delivery tip) partway through, so the movements log isn't sales-only.
    if (dayInvoices.length > 3) {
      const midTime = dayInvoices[Math.floor(dayInvoices.length / 2)].date;
      movements.push({ id: uid('shmv'), kind: 'PAY_OUT', amount: 20, note: 'أجرة توصيل طارئة', at: midTime, by: CASHIER });
    }
    shift.movements = movements.sort((a, b) => a.at.localeCompare(b.at));

    const expectedCash = round2(
      openingFloat +
        sum(movements.filter((m) => m.kind === 'SALE_CASH'), (m) => m.amount) -
        sum(movements.filter((m) => m.kind === 'REFUND_CASH'), (m) => m.amount) -
        sum(movements.filter((m) => m.kind === 'PAY_OUT'), (m) => m.amount),
    );
    // Alternate a small over/short/exact variance across the seeded shifts so the manager screen and
    // invariant 11 both see over, short AND exact-match cases.
    const variancePattern = [2.5, -3, 0, 1];
    const counted = round2(expectedCash + variancePattern[seq % variancePattern.length]);
    seq++;

    closeShift(shift.id, { countedCash: counted, handoverMode: 'HANDOVER', note: 'إغلاق تلقائي — بيانات تجريبية' }, CASHIER, closeAt.toISOString());
  }
}
