import type { PartyStatementRow } from '@/modules/parties/types';
import { db } from '../db';
import { round2, sum } from '../utils';
import { accountFor } from './accounts';

/**
 * Party balances from the ledger (docs/v2/02-accounting-review.md C2): a party's balance is
 * Σ(debit − credit) of the journal lines tagged with that party on the receivable/payable control
 * account — not by summing open documents. This is what makes "AR GL = Σ customers" / "AP GL = Σ
 * suppliers" hold structurally: every line on the control account carries a party (enforced by
 * B1's manual-entry rule + every system posting tagging `partyKind`/`partyId`), so the control
 * account's balance and the sum of party balances are the same total by construction.
 */
function partyLedgerLines(kind: 'customer' | 'supplier', partyId: string) {
  const accountId = accountFor(kind === 'customer' ? 'receivable' : 'payable').id;
  return db.journalEntries
    .flatMap((e) => e.lines.map((l) => ({ ...l, entry: e })))
    .filter((l) => l.accountId === accountId && l.partyKind === kind && l.partyId === partyId);
}

export function customerBalance(customerId: string): number {
  return round2(sum(partyLedgerLines('customer', customerId), (l) => l.debit - l.credit));
}

export function supplierBalance(supplierId: string): number {
  return round2(sum(partyLedgerLines('supplier', supplierId), (l) => l.credit - l.debit));
}

/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §2 "Realized FX" worked example):
 * the party's balance in ITS OWN foreign currency, from `amountFc` on the same control-account
 * ledger lines `customerBalance`/`supplierBalance` read (only lines actually tagged with an FC
 * amount count — the FX gain/loss line itself never carries one). This is what should net to
 * exactly 0 once an FC invoice is fully settled by an FC receipt, regardless of what the base
 * currency did to get there.
 */
export function customerBalanceFc(customerId: string): number {
  return round2(sum(partyLedgerLines('customer', customerId).filter((l) => l.amountFc !== undefined), (l) => (l.debit > 0 ? (l.amountFc ?? 0) : -(l.amountFc ?? 0))));
}

export function supplierBalanceFc(supplierId: string): number {
  return round2(sum(partyLedgerLines('supplier', supplierId).filter((l) => l.amountFc !== undefined), (l) => (l.credit > 0 ? (l.amountFc ?? 0) : -(l.amountFc ?? 0))));
}

function withRunningBalance(rows: Omit<PartyStatementRow, 'balance'>[], sign: 1 | -1): PartyStatementRow[] {
  rows.sort((a, b) => a.date.localeCompare(b.date));
  let balance = 0;
  return rows.map((r) => {
    balance = round2(balance + sign * (r.debit - r.credit));
    return { ...r, balance };
  });
}

/**
 * Customer statement: one row per ledger line on the receivable control account tagged with this
 * customer (invoices are debits; payments and returns settled against the receivable are
 * credits). The final running balance equals `customerBalance()` by construction.
 */
export function customerStatement(customerId: string): PartyStatementRow[] {
  const rows: Omit<PartyStatementRow, 'balance'>[] = partyLedgerLines('customer', customerId).map((l) => {
    const ref = l.entry.sourceRef;
    // v2 phase 5 (docs/v2/05-onboarding.md §4 "On the party: Statement — the balance appears as a
    // رصيد افتتاحي row"): both the wizard's opening entry and a party-form "رصيد سابق" entry carry
    // `sourceRef.kind === 'opening'`.
    const kind: PartyStatementRow['kind'] = ref?.kind === 'refund' ? 'refund' : ref?.kind === 'payment' ? 'payment' : ref?.kind === 'opening' ? 'opening' : 'invoice';
    return {
      id: l.id,
      date: l.entry.date,
      kind,
      refId: ref?.id ?? l.entry.id,
      number: ref?.number ?? l.entry.number,
      description: l.description ?? l.entry.description,
      debit: l.debit,
      credit: l.credit,
    };
  });
  return withRunningBalance(rows, 1);
}

/**
 * Supplier statement (AP view): one row per ledger line on the payable control account tagged
 * with this supplier (purchases are credits — we owe more; payments and returns settled against
 * the payable are debits). Balance = credit − debit = `supplierBalance()`.
 */
export function supplierStatement(supplierId: string): PartyStatementRow[] {
  const rows: Omit<PartyStatementRow, 'balance'>[] = partyLedgerLines('supplier', supplierId).map((l) => {
    const ref = l.entry.sourceRef;
    const kind: PartyStatementRow['kind'] =
      ref?.kind === 'purchaseReturn' ? 'purchaseReturn' : ref?.kind === 'payment' ? 'payment' : ref?.kind === 'opening' ? 'opening' : 'purchaseOrder';
    return {
      id: l.id,
      date: l.entry.date,
      kind,
      refId: ref?.id ?? l.entry.id,
      number: ref?.number ?? l.entry.number,
      description: l.description ?? l.entry.description,
      debit: l.debit,
      credit: l.credit,
    };
  });
  return withRunningBalance(rows, -1);
}
