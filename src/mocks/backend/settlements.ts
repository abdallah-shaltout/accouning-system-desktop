import type { CardSettlement, CardSettlementInput, UnsettledTenderGroup } from '@/modules/vouchers/types';
import { db, nextNumber } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, localDateKey, round2, uid } from '../utils';
import { logActivity, postJournal, type PostingLine } from './core';

/**
 * Card settlement (docs/v2/09-purchases-payments-expenses.md §2 "Card settlement") — completes
 * Phase 3's `TODO(phase 8)` in src/mocks/backend/sales.ts: card/wallet tenders post to their
 * clearing account at sale time; this voucher clears that balance when the bank deposit arrives.
 * Post: Dr bank, Dr card fees / Cr clearing (cardClearing or walletClearing, by payment method).
 */

/** Tenders already covered by a posted settlement, by (date, paymentMethodId) — so they don't offer twice. */
function settledKeys(): Set<string> {
  const keys = new Set<string>();
  for (const s of db.cardSettlements) for (const g of s.groups) keys.add(`${g.date}::${g.paymentMethodId}`);
  return keys;
}

/** Card/wallet tenders grouped by day + method, with totals — the settlement screen's pick list. */
export function unsettledTenderGroups(): UnsettledTenderGroup[] {
  const clearingMethods = db.paymentMethods.filter((m) => m.accountRole === 'cardClearing' || m.accountRole === 'walletClearing');
  if (!clearingMethods.length) return [];
  const methodById = new Map(clearingMethods.map((m) => [m.id, m]));
  const already = settledKeys();

  const totals = new Map<string, UnsettledTenderGroup>();
  for (const inv of db.invoices) {
    if (inv.status !== 'COMPLETED' || !inv.tenders?.length) continue;
    const day = localDateKey(inv.date);
    for (const t of inv.tenders) {
      const method = methodById.get(t.paymentMethodId);
      if (!method) continue;
      const key = `${day}::${method.id}`;
      if (already.has(key)) continue;
      const existing = totals.get(key);
      if (existing) {
        existing.total = round2(existing.total + t.amount);
        existing.tenderCount += 1;
      } else {
        totals.set(key, {
          date: day,
          paymentMethodId: method.id,
          paymentMethodName: method.name,
          accountRole: method.accountRole as 'cardClearing' | 'walletClearing',
          total: round2(t.amount),
          tenderCount: 1,
        });
      }
    }
  }
  return [...totals.values()].sort((a, b) => b.date.localeCompare(a.date) || a.paymentMethodName.localeCompare(b.paymentMethodName));
}

/** Posts a settlement: Dr bank + Dr cardFees / Cr cardClearing-or-walletClearing (fee = gross − deposit). */
export function recordCardSettlement(input: CardSettlementInput, userId: string): CardSettlement {
  if (!input.groups.length) throw new ApiError('اختر يوماً واحداً على الأقل للتسوية');
  if (!(input.depositAmount >= 0)) throw new ApiError('أدخل مبلغ الإيداع البنكي');

  const available = new Map(unsettledTenderGroups().map((g) => [`${g.date}::${g.paymentMethodId}`, g]));
  const selected: { date: string; paymentMethodId: string; amount: number; role: 'cardClearing' | 'walletClearing' }[] = [];
  for (const g of input.groups) {
    const found = available.get(`${g.date}::${g.paymentMethodId}`);
    if (!found) throw new ApiError('أحد العناصر المختارة غير متاح للتسوية (ربما تمت تسويته بالفعل)', 'CONFLICT');
    selected.push({ date: g.date, paymentMethodId: g.paymentMethodId, amount: found.total, role: found.accountRole });
  }
  const grossAmount = round2(selected.reduce((a, s) => a + s.amount, 0));
  const feeAmount = round2(grossAmount - input.depositAmount);
  if (feeAmount < -0.005) throw new ApiError('مبلغ الإيداع أكبر من إجمالي العمليات المختارة');

  const settlement: CardSettlement = {
    id: uid('stl'),
    number: nextNumber('cardSettlement'),
    date: input.date,
    groups: selected.map((s) => ({ date: s.date, paymentMethodId: s.paymentMethodId, amount: s.amount })),
    grossAmount,
    depositAmount: round2(input.depositAmount),
    feeAmount,
    note: input.note,
    createdBy: userId,
  };
  mutate(() => db.cardSettlements.push(settlement));

  // Group by clearing role so cardClearing and walletClearing each get their own credit line when a
  // settlement mixes both (rare, but the picker doesn't forbid it).
  const byRole = new Map<'cardClearing' | 'walletClearing', number>();
  for (const s of selected) byRole.set(s.role, round2((byRole.get(s.role) ?? 0) + s.amount));

  const lines: PostingLine[] = [{ role: 'bank', debit: settlement.depositAmount }];
  if (feeAmount > 0) lines.push({ role: 'cardFees', debit: feeAmount });
  for (const [role, amount] of byRole) lines.push({ role, credit: amount });
  if (feeAmount < 0) lines.push({ role: 'bank', debit: -feeAmount }); // deposit exceeded gross (rare: a correction/bonus) — extra goes to bank too

  postJournal({
    date: input.date,
    description: `تسوية بطاقات/محافظ ${settlement.number} — ${selected.length} مجموعة`,
    type: 'SYSTEM',
    sourceRef: { kind: 'settlement', id: settlement.id, number: settlement.number },
    lines,
    createdBy: userId,
  });

  logActivity('payment', `تسوية بطاقات ${settlement.number} — إيداع ${settlement.depositAmount.toFixed(2)} وعمولة ${feeAmount.toFixed(2)}`, userId, input.date, `/payments/settlements/${settlement.id}`);
  emit('ledger:changed');
  return settlement;
}

export function getCardSettlementById(id: string): CardSettlement {
  const settlement = db.cardSettlements.find((s) => s.id === id);
  if (!settlement) throw new ApiError('سند التسوية غير موجود', 'NOT_FOUND');
  return settlement;
}

/** Pre-filled fee estimate from the payment method's fee % (docs/v2 §2 "pre-filled from fee %"). */
export function estimatedFeeFor(groups: UnsettledTenderGroup[]): number {
  return round2(
    groups.reduce((a, g) => {
      const method = db.paymentMethods.find((m) => m.id === g.paymentMethodId);
      return a + (g.total * (method?.feePct ?? 0)) / 100;
    }, 0),
  );
}
