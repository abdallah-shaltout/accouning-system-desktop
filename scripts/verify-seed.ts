import { db } from '../src/mocks';
import { customerBalance, supplierBalance, customerStatement, supplierStatement } from '../src/mocks/backend/balances';

const bal = (code: string) => {
  const id = `acc-${code}`;
  return Math.round(db.journalEntries.flatMap(e => e.lines).filter(l => l.accountId === id).reduce((a, l) => a + l.debit - l.credit, 0) * 100) / 100;
};
const r2 = (n: number) => Math.round(n * 100) / 100;
let dr = 0, cr = 0;
for (const e of db.journalEntries) { dr += e.totalDebit; cr += e.totalCredit; if (Math.abs(e.totalDebit - e.totalCredit) > 0.001) console.log('UNBALANCED', e.number); }
console.log('entries', db.journalEntries.length, 'invoices', db.invoices.length, 'refunds', db.refunds.length, 'POs', db.purchaseOrders.length, 'payments', db.payments.length, 'adjustments', db.stockAdjustments.length, 'movements', db.stockMovements.length, 'activity', db.activity.length);
console.log('total Dr/Cr', r2(dr), r2(cr));
const arSum = r2(db.customers.reduce((a, c) => a + customerBalance(c.id), 0));
const apSum = r2(db.suppliers.reduce((a, s) => a + supplierBalance(s.id), 0));
console.log('AR GL', bal('1130'), 'Σcustomers', arSum);
console.log('AP GL', -bal('2100'), 'Σsuppliers', apSum);
for (const c of db.customers) { const st = customerStatement(c.id); const last = st.at(-1)?.balance ?? 0; if (Math.abs(last - customerBalance(c.id)) > 0.01) console.log('STMT MISMATCH cust', c.id, last, customerBalance(c.id)); }
for (const s of db.suppliers) { const st = supplierStatement(s.id); const last = st.at(-1)?.balance ?? 0; if (Math.abs(last - supplierBalance(s.id)) > 0.01) console.log('STMT MISMATCH sup', s.id, last, supplierBalance(s.id)); }
const invValue = r2(db.products.reduce((a, p) => a + (p.type === 'product' ? p.stockQty * p.costPrice : 0), 0));
console.log('Inventory GL', bal('1140'), 'Σqty×cost', invValue);
console.log('Cash', bal('1110'), 'Bank', bal('1120'), 'VAT out', -bal('2150'), 'VAT in', bal('1150'));
console.log('negative stock', db.products.filter(p => p.stockQty < 0).map(p => p.name));
console.log('low stock', db.products.filter(p => p.type === 'product' && p.stockQty <= (p.minStock ?? 0)).length);
const todayKey = new Date().toDateString();
console.log('today invoices', db.invoices.filter(i => new Date(i.date).toDateString() === todayKey).length);
// movement ordering check: per product running balance matches
for (const p of db.products) {
  const mv = db.stockMovements.filter(m => m.productId === p.id);
  let q = 0; let ok = true;
  for (const m of mv) { q = r2(q + m.qtyChange); if (Math.abs(q - (m.balanceAfter ?? 0)) > 0.001) ok = false; }
  if (!ok || Math.abs(q - p.stockQty) > 0.001) console.log('MOVEMENT MISMATCH', p.name);
}
const unsorted = db.stockMovements.some((m, i, a) => i > 0 && a[i-1].date > m.date);
console.log('movements chronological:', !unsorted);
console.log('PO statuses', db.purchaseOrders.map(p => p.status).join(','));
console.log('invoice statuses', Object.entries(db.invoices.reduce((a: any, i) => (a[i.status + '/' + i.paymentStatus] = (a[i.status + '/' + i.paymentStatus] ?? 0) + 1, a), {})));
