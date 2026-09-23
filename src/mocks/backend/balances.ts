import type { PartyStatementRow } from '@/modules/parties/types';
import { invoiceOutstanding } from '@/modules/invoices/helpers/totals';
import { db } from '../db';
import { round2, sum } from '../utils';
import { purchaseOutstanding } from './purchases';

export function customerBalance(customerId: string): number {
  return sum(
    db.invoices.filter((i) => i.customerId === customerId && i.status !== 'DRAFT'),
    invoiceOutstanding,
  );
}

export function supplierBalance(supplierId: string): number {
  return sum(
    db.purchaseOrders.filter((p) => p.supplierId === supplierId && p.status === 'CONFIRMED'),
    purchaseOutstanding,
  );
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
 * Customer statement: invoices are debits; payments (at sale + later) and returns settled
 * against the receivable are credits. The final balance equals `customerBalance()`.
 */
export function customerStatement(customerId: string): PartyStatementRow[] {
  const rows: Omit<PartyStatementRow, 'balance'>[] = [];
  const invoices = db.invoices.filter((i) => i.customerId === customerId && i.status !== 'DRAFT');
  for (const inv of invoices) {
    rows.push({ id: `${inv.id}-d`, date: inv.date, kind: 'invoice', refId: inv.id, number: inv.number, description: 'فاتورة مبيعات', debit: inv.grandTotal, credit: 0 });
    const laterPayments = sum(db.payments.filter((p) => p.targetRef === inv.id), (p) => p.amount);
    const paidAtSale = round2(inv.paidAmount - laterPayments);
    if (paidAtSale > 0) {
      rows.push({ id: `${inv.id}-p`, date: inv.date, kind: 'invoice', refId: inv.id, number: inv.number, description: 'مدفوع عند البيع', debit: 0, credit: paidAtSale });
    }
  }
  for (const ref of db.refunds.filter((r) => invoices.some((i) => i.id === r.invoiceId))) {
    if (ref.settledToReceivable > 0) {
      rows.push({ id: ref.id, date: ref.date, kind: 'refund', refId: ref.invoiceId, number: ref.number, description: 'مرتجع مبيعات', debit: 0, credit: ref.settledToReceivable });
    }
  }
  for (const pay of db.payments.filter((p) => p.type === 'RECEIVED' && p.targetId === customerId)) {
    rows.push({ id: pay.id, date: pay.date, kind: 'payment', refId: pay.id, number: pay.number, description: `سند قبض — ${pay.targetRefNumber ?? ''}`, debit: 0, credit: pay.amount });
  }
  return withRunningBalance(rows, 1);
}

/**
 * Supplier statement (AP view): purchase orders are credits (we owe more); payments and
 * returns settled against the payable are debits. Balance = credit − debit = `supplierBalance()`.
 */
export function supplierStatement(supplierId: string): PartyStatementRow[] {
  const rows: Omit<PartyStatementRow, 'balance'>[] = [];
  const orders = db.purchaseOrders.filter((p) => p.supplierId === supplierId && p.status === 'CONFIRMED');
  for (const po of orders) {
    rows.push({ id: po.id, date: po.date, kind: 'purchaseOrder', refId: po.id, number: po.number, description: 'أمر شراء', debit: 0, credit: po.grandTotal });
  }
  for (const ret of db.purchaseReturns.filter((r) => r.supplierId === supplierId && r.settledToPayable > 0)) {
    rows.push({ id: ret.id, date: ret.date, kind: 'purchaseReturn', refId: ret.purchaseOrderId, number: ret.number, description: 'مرتجع مشتريات', debit: ret.settledToPayable, credit: 0 });
  }
  for (const pay of db.payments.filter((p) => p.type === 'PAID' && p.targetId === supplierId)) {
    rows.push({ id: pay.id, date: pay.date, kind: 'payment', refId: pay.id, number: pay.number, description: `سند صرف — ${pay.targetRefNumber ?? ''}`, debit: pay.amount, credit: 0 });
  }
  return withRunningBalance(rows, -1);
}
