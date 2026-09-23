/**
 * Currency revaluation wizard (docs/v2/10-branches-currencies-cost-centers.md §2 "Unrealized FX",
 * optional but built per this phase's checklist): revalues every open FC balance (party sub-ledgers
 * with `amountFc`, FC cash/bank accounts) at a chosen rate, posts the difference to FX gain/loss,
 * and immediately posts the exact mirror entry dated the first day of the next month so it
 * "auto-reverses" without needing a second manual step.
 */
import { db } from '../db';
import { ApiError, round2, sum } from '../utils';
import { activeCurrencies } from './currency';
import { logActivity, postJournal, type PostingLine } from './core';

export interface FcBalanceRow {
  kind: 'customer' | 'supplier' | 'account';
  id: string;
  name: string;
  currency: string;
  fcBalance: number;
  /** Current base-currency carrying value (Σ amountFc-tagged lines' debit−credit in base). */
  baseBalance: number;
  revaluedBase: number;
  gainLoss: number;
}

/** Every open FC balance the wizard can revalue, for the currency picked (defaults to every active currency when omitted). */
export function openFcBalances(rates: Record<string, number>): FcBalanceRow[] {
  const rows: FcBalanceRow[] = [];

  function fcLinesFor(role: 'receivable' | 'payable', kind: 'customer' | 'supplier') {
    const accountId = db.accounts.find((a) => a.systemRole === role)?.id;
    if (!accountId) return [];
    return db.journalEntries.flatMap((e) => e.lines).filter((l) => l.accountId === accountId && l.partyKind === kind && l.amountFc !== undefined && l.currency);
  }

  for (const [kind, role] of [
    ['customer', 'receivable'],
    ['supplier', 'payable'],
  ] as const) {
    const lines = fcLinesFor(role, kind);
    const byParty = new Map<string, typeof lines>();
    for (const l of lines) byParty.set(l.partyId!, [...(byParty.get(l.partyId!) ?? []), l]);
    for (const [partyId, partyLines] of byParty) {
      const currency = partyLines[0].currency!;
      const rate = rates[currency];
      if (!rate) continue;
      const fcBalance = round2(sum(partyLines, (l) => (kind === 'customer' ? (l.debit > 0 ? (l.amountFc ?? 0) : -(l.amountFc ?? 0)) : l.credit > 0 ? (l.amountFc ?? 0) : -(l.amountFc ?? 0))));
      const baseBalance = round2(sum(partyLines, (l) => l.debit - l.credit) * (kind === 'supplier' ? -1 : 1));
      if (Math.abs(fcBalance) < 0.005) continue;
      const revaluedBase = round2(fcBalance * rate);
      const party = (kind === 'customer' ? db.customers : db.suppliers).find((p) => p.id === partyId);
      rows.push({ kind, id: partyId, name: party?.name ?? partyId, currency, fcBalance, baseBalance, revaluedBase, gainLoss: round2(revaluedBase - baseBalance) });
    }
  }

  // FC cash/bank accounts: revalue the account's own posted FC balance (Σ amountFc on its lines).
  for (const account of db.accounts.filter((a) => a.currency && !a.isGroup)) {
    const rate = rates[account.currency!];
    if (!rate) continue;
    const lines = db.journalEntries.flatMap((e) => e.lines).filter((l) => l.accountId === account.id && l.amountFc !== undefined);
    if (!lines.length) continue;
    const fcBalance = round2(sum(lines, (l) => l.debit - l.credit) === 0 ? 0 : sum(lines, (l) => (l.amountFc ?? 0) * (l.debit > 0 ? 1 : -1)));
    const baseBalance = round2(sum(lines, (l) => l.debit - l.credit));
    if (Math.abs(fcBalance) < 0.005) continue;
    const revaluedBase = round2(fcBalance * rate);
    rows.push({ kind: 'account', id: account.id, name: account.name, currency: account.currency!, fcBalance, baseBalance, revaluedBase, gainLoss: round2(revaluedBase - baseBalance) });
  }

  return rows;
}

/** Default rates: the latest rate on record for every active currency. */
export function defaultRevaluationRates(asOf?: string): Record<string, number> {
  const rates: Record<string, number> = {};
  for (const c of activeCurrencies()) {
    const latest = db.exchangeRates.filter((r) => r.currency === c.code && r.date <= (asOf ?? '9999-99-99')).sort((a, b) => b.date.localeCompare(a.date))[0];
    if (latest) rates[c.code] = latest.rate;
    else if (c.fixed && c.fixedRate) rates[c.code] = c.fixedRate;
  }
  return rates;
}

/** First day of the month after `date` — the auto-reversal's date. */
function firstOfNextMonth(date: string): string {
  const d = new Date(date);
  const y = d.getFullYear();
  const m = d.getMonth(); // 0-11; +1 below rolls into next month, Date normalizes a December overflow into next January
  const next = new Date(y, m + 1, 1);
  return `${next.getFullYear()}-${String(next.getMonth() + 1).padStart(2, '0')}-01`;
}

/**
 * Posts the revaluation entry (Dr/Cr the party's receivable/payable or the FC account, vs FX
 * gain/loss) dated `date`, then immediately posts the exact mirror entry dated the first day of
 * the next month — this is the "auto-reverses on the first day of the next period" behavior.
 */
export function postRevaluation(date: string, rates: Record<string, number>, userId: string): { entryId: string; reversalEntryId: string; rows: FcBalanceRow[] } {
  const rows = openFcBalances(rates).filter((r) => Math.abs(r.gainLoss) >= 0.01);
  if (!rows.length) throw new ApiError('لا توجد أرصدة عملات أجنبية بحاجة لإعادة تقييم بهذه الأسعار');

  const lines: PostingLine[] = [];
  let totalGain = 0;
  for (const row of rows) {
    totalGain = round2(totalGain + row.gainLoss);
    const desc = `إعادة تقييم ${row.currency} — ${row.name}`;
    if (row.kind === 'account') {
      lines.push(row.gainLoss > 0 ? { accountId: row.id, debit: row.gainLoss, description: desc } : { accountId: row.id, credit: -row.gainLoss, description: desc });
    } else if (row.kind === 'customer') {
      lines.push(row.gainLoss > 0 ? { role: 'receivable', debit: row.gainLoss, partyKind: 'customer', partyId: row.id, description: desc } : { role: 'receivable', credit: -row.gainLoss, partyKind: 'customer', partyId: row.id, description: desc });
    } else {
      lines.push(row.gainLoss > 0 ? { role: 'payable', credit: row.gainLoss, partyKind: 'supplier', partyId: row.id, description: desc } : { role: 'payable', debit: -row.gainLoss, partyKind: 'supplier', partyId: row.id, description: desc });
    }
  }
  if (totalGain > 0) lines.push({ role: 'fxGain', credit: totalGain, description: 'صافي إعادة تقييم العملات' });
  else if (totalGain < 0) lines.push({ role: 'fxLoss', debit: -totalGain, description: 'صافي إعادة تقييم العملات' });

  const entry = postJournal({
    date,
    description: `إعادة تقييم العملات بتاريخ ${date}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'fxReval', id: `fxreval-${date}`, number: `FXR-${date}` },
    lines,
    createdBy: userId,
  });

  // Auto-reversal: the exact mirror, dated the first day of the next period.
  const reversalDate = firstOfNextMonth(date);
  const reversal = postJournal({
    date: reversalDate,
    description: `عكس إعادة تقييم العملات بتاريخ ${date}`,
    type: 'SYSTEM',
    sourceRef: { kind: 'fxReval', id: `fxreval-${date}-rev`, number: `FXR-${date}` },
    lines: entry.lines.map((l) => ({ accountId: l.accountId, description: l.description, debit: l.credit, credit: l.debit, partyKind: l.partyKind, partyId: l.partyId })),
    createdBy: userId,
    allowClosedPeriod: true,
  });

  logActivity('journal', `إعادة تقييم العملات بتاريخ ${date} (${rows.length} رصيد)`, userId, date, `/accounting/journal/${entry.id}`);
  return { entryId: entry.id, reversalEntryId: reversal.id, rows };
}
