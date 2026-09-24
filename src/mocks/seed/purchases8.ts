import { db } from '../db';
import { receivePurchase, savePurchase, sendPurchaseToSupplier } from '../backend/purchases';
import { recordExpense, saveExpenseCategory, saveRecurringExpense } from '../backend/expenses';
import { recordOwnerVoucher, recordPaymentVoucher, recordReceiptVoucher, recordTransferVoucher } from '../backend/vouchers';
import { recordCardSettlement, unsettledTenderGroups } from '../backend/settlements';
import { round2 } from '../utils';

/**
 * v2 phase 8 seed (docs/v2/09-purchases-payments-expenses.md): purchase v2 demo data (landed costs,
 * an ORDERED-not-yet-received PO, a non-VAT supplier receipt), expense categories + sample
 * expenses + a recurring template, sample general vouchers, and one settled card-settlement voucher
 * so the settlement screen and `verify:mocks`' card-clearing invariant both have real data to check.
 *
 * Runs after `seedHistory()` (needs suppliers/products/accounts/payment methods and — for the card
 * settlement — posted invoices with card tenders — already seeded).
 */
export function seedPurchases8(now: Date, adminId: string, accountantId: string, managerId: string): void {
  seedExpenseCategoriesAndExpenses(now, accountantId);
  seedPurchaseV2Demo(now, managerId, accountantId);
  seedVouchers(now, adminId, accountantId);
  seedCardSettlement(now, accountantId);
}

function accountId(code: string): string {
  const account = db.accounts.find((a) => a.code === code);
  if (!account) throw new Error(`seed/purchases8.ts: account ${code} missing from the v2 chart of accounts`);
  return account.id;
}

/**
 * `seedHistory()` (and `seedShifts()`) already fill `db.stockMovements`/`db.journalEntries` up to
 * `now` itself (its last stocktake/sale/shift-close events land within minutes of `now`), and
 * `verify:mocks`' inventory invariant checks that stock movements stay in *insertion* order — so
 * every movement this file adds must be timestamped strictly after everything already seeded, not
 * just "N days ago" by calendar date. All purchase-receiving here therefore lands within the last
 * few minutes before `now`, in strictly increasing order, instead of reusing history's `daysAgo`.
 */
let cursorMinutes = 0;
function nextTimestamp(now: Date): string {
  cursorMinutes += 1;
  const d = new Date(now.getTime() + cursorMinutes * 1000);
  return d.toISOString();
}

/** Non-stock-moving events (expenses, vouchers, a card settlement) can still read naturally as "N days ago" on screen. */
function daysAgo(now: Date, days: number): string {
  const d = new Date(now);
  d.setDate(d.getDate() - days);
  return d.toISOString();
}

// ---------------------------------------------------------------------------------------------
// Expenses (§4)
// ---------------------------------------------------------------------------------------------

function seedExpenseCategoriesAndExpenses(now: Date, userId: string): void {
  const rent = saveExpenseCategory({ name: 'الإيجار', accountId: accountId('6220'), active: true });
  const electricity = saveExpenseCategory({ name: 'الكهرباء والمياه', accountId: accountId('6230'), active: true });
  const internet = saveExpenseCategory({ name: 'الاتصالات والإنترنت', accountId: accountId('6240'), active: true });
  const packaging = saveExpenseCategory({ name: 'مواد تغليف وأكياس', accountId: accountId('6130'), active: true });
  const maintenance = saveExpenseCategory({ name: 'الصيانة والإصلاح', accountId: accountId('6250'), active: true });
  saveExpenseCategory({ name: 'مصروفات متنوعة', accountId: accountId('6390'), active: true });

  const cashMethod = db.paymentMethods.find((m) => m.accountRole === 'cash')!;
  const bankMethod = db.paymentMethods.find((m) => m.accountRole === 'bank')!;

  recordExpense(
    { date: daysAgo(now, 40), categoryId: internet.id, amount: 345, isTaxInvoice: true, taxId: 'tax-vat-in', supplierInvoiceNo: 'STC-88213', paidFrom: { kind: 'method', paymentMethodId: bankMethod.id }, description: 'اشتراك الإنترنت الشهري' },
    userId,
  );
  recordExpense(
    { date: daysAgo(now, 25), categoryId: packaging.id, amount: 210.5, isTaxInvoice: true, taxId: 'tax-vat-in', paidFrom: { kind: 'method', paymentMethodId: cashMethod.id }, description: 'أكياس وعبوات تغليف' },
    userId,
  );
  recordExpense(
    { date: daysAgo(now, 12), categoryId: maintenance.id, amount: 480, isTaxInvoice: false, paidFrom: { kind: 'method', paymentMethodId: cashMethod.id }, description: 'صيانة مكيفات المحل' },
    userId,
  );
  recordExpense(
    { date: daysAgo(now, 5), categoryId: electricity.id, amount: 1560, isTaxInvoice: true, taxId: 'tax-vat-in', paidFrom: { kind: 'method', paymentMethodId: bankMethod.id }, description: 'فاتورة الكهرباء' },
    userId,
  );

  // Recurring rent template, due next month — exercises the due-list on the expenses page.
  const nextMonth = new Date(now);
  nextMonth.setMonth(nextMonth.getMonth() + 1, 1);
  saveRecurringExpense({
    name: 'إيجار المحل الشهري',
    categoryId: rent.id,
    amount: 9000,
    isTaxInvoice: false,
    paidFrom: { kind: 'method', paymentMethodId: bankMethod.id },
    description: 'إيجار المحل',
    day: 1,
    nextDate: nextMonth.toISOString().slice(0, 10),
    autoPost: false,
    active: true,
  });
  // One already overdue, so the "due today" list has something to show on first load.
  saveRecurringExpense({
    name: 'اشتراك الإنترنت الشهري',
    categoryId: internet.id,
    amount: 345,
    isTaxInvoice: true,
    taxId: 'tax-vat-in',
    paidFrom: { kind: 'method', paymentMethodId: bankMethod.id },
    description: 'اشتراك الإنترنت الشهري',
    day: 1,
    nextDate: daysAgo(now, 2).slice(0, 10),
    autoPost: false,
    active: true,
  });
}

// ---------------------------------------------------------------------------------------------
// Purchases v2 (§1): landed costs, ORDERED-not-received, non-VAT supplier
// ---------------------------------------------------------------------------------------------

function seedPurchaseV2Demo(now: Date, managerId: string, accountantId: string): void {
  // 1) A received purchase order WITH landed costs (freight, spread by value) — proves the GL-
  // matches-stockValue invariant with landed cost folded in (review E4).
  const landedDate = nextTimestamp(now);
  const landedPo = savePurchase(
    {
      supplierId: 'sup-1',
      date: landedDate,
      note: 'شحنة قماش مع تكلفة شحن',
      confirm: false,
      landedCosts: [{ label: 'شحن من المصنع', amount: 350, spreadBy: 'value' }],
      lines: [
        { productId: 'prd-1', qty: 40, costPrice: 45 },
        { productId: 'prd-2', qty: 30, costPrice: 60 },
      ],
    },
    managerId,
  );
  receivePurchase(landedPo.id, { date: landedDate, lines: landedPo.lines.map((l) => ({ productId: l.productId, receivedQty: l.qty })), supplierInvoiceNo: 'INV-SUP1-2201', supplierInvoiceDate: landedDate }, managerId);

  // 2) An ORDERED PO — sent to the supplier but not yet received, so the receiving screen and
  // "مرسل للمورد" status both have a real row to exercise.
  const orderedPo = savePurchase(
    {
      supplierId: 'sup-2',
      date: nextTimestamp(now),
      note: 'طلبية موسم جديد',
      confirm: false,
      lines: [{ productId: 'prd-8', qty: 25, costPrice: 55 }],
    },
    managerId,
  );
  sendPurchaseToSupplier(orderedPo.id, managerId);

  // 3) A non-VAT supplier receipt (sup-5 has no vatNumber) — input VAT gets folded into cost instead
  // of claimed (review E3), and a short delivery creates a backorder draft (§1 "إنشاء أمر متبقٍ").
  const nonVatDate = nextTimestamp(now);
  const nonVatPo = savePurchase(
    {
      supplierId: 'sup-5',
      date: nonVatDate,
      note: 'طلبية ملابس أطفال',
      confirm: false,
      lines: [{ productId: 'prd-15', qty: 30, costPrice: 20 }],
    },
    managerId,
  );
  receivePurchase(nonVatPo.id, { date: nonVatDate, lines: [{ productId: 'prd-15', receivedQty: 22 }], createBackorder: true }, managerId);

  // 4) A plain received PO missing its supplier invoice number/date — the warning banner has
  // something real to show.
  const warnDate = nextTimestamp(now);
  const warnPo = savePurchase({ supplierId: 'sup-3', date: warnDate, confirm: false, lines: [{ productId: 'prd-22', qty: 15, costPrice: 70 }] }, accountantId);
  receivePurchase(warnPo.id, { date: warnDate, lines: [{ productId: 'prd-22', receivedQty: 15 }] }, accountantId);
}

// ---------------------------------------------------------------------------------------------
// General vouchers (§5)
// ---------------------------------------------------------------------------------------------

function seedVouchers(now: Date, adminId: string, accountantId: string): void {
  const cash = db.paymentMethods.find((m) => m.accountRole === 'cash')!;
  const otherIncome = accountId('4300');
  const bankFees = accountId('6280');
  const cashAcc = accountId('1110');
  const bankAcc = accountId('1120');

  recordReceiptVoucher(
    { date: daysAgo(now, 20), amount: 150, description: 'بيع كراتين فارغة (خردة)', paymentMethodId: cash.id, creditAccountId: otherIncome },
    accountantId,
  );
  recordPaymentVoucher(
    { date: daysAgo(now, 15), amount: 60, description: 'رسوم تحويل بنكي', paymentMethodId: cash.id, debitAccountId: bankFees },
    accountantId,
  );
  recordTransferVoucher(
    { date: daysAgo(now, 10), amount: 5000, description: 'إيداع نقدية الصندوق بالبنك', sourceAccountId: cashAcc, destinationAccountId: bankAcc },
    accountantId,
  );
  recordOwnerVoucher(
    { date: daysAgo(now, 30), amount: 3000, description: 'مسحوبات شخصية للمالك', direction: 'drawings', cashAccountId: cashAcc },
    adminId,
  );
}

// ---------------------------------------------------------------------------------------------
// Card settlement (§2) — completes the follow-up Phase 3 left for Phase 8; settles the oldest half
// of the unsettled card/wallet tenders history.ts's sales already posted, so both an open
// (unsettled) balance and a settled example exist.
// ---------------------------------------------------------------------------------------------

function seedCardSettlement(now: Date, userId: string): void {
  const groups = unsettledTenderGroups();
  if (!groups.length) return;
  const half = groups.slice(Math.ceil(groups.length / 2));
  if (!half.length) return;
  const gross = round2(half.reduce((a, g) => a + g.total, 0));
  const fee = round2(gross * 0.015);
  recordCardSettlement(
    {
      date: daysAgo(now, 7),
      groups: half.map((g) => ({ date: g.date, paymentMethodId: g.paymentMethodId })),
      depositAmount: round2(gross - fee),
      note: 'تسوية دفعة الإيداع البنكي',
    },
    userId,
  );
}
