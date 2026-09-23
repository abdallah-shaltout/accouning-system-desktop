/**
 * v2 phase 9 seed (docs/v2/10-branches-currencies-cost-centers.md): a second branch, one USD
 * customer, one cost-center split on a manual journal line, and the realized-FX worked example
 * (USD invoice + USD receipt) — so every dimension this phase activates is actually demonstrable
 * in the demo data, not just built-and-empty. Runs last (after `seedPurchases8`): it needs the
 * main branch's cash account (created by `seedAccounts`/onboarding-equivalent bootstrap below),
 * every other area's fixtures, and strictly-increasing timestamps after everything else.
 */
import { db } from '../db';
import { createBranch, createCostCenter, MAIN_BRANCH_ID } from '../backend/branches';
import { createCurrency, saveExchangeRate } from '../backend/currency';
import { recordManualJournal, splitLineByCostCenters } from '../backend/journal';
import { recordSale } from '../backend/sales';
import { recordPayment } from '../backend/payments';
import { draftTransfer, sendTransfer, receiveTransfer, branchStockQty } from '../backend/transfers';

function accountId(code: string): string {
  const account = db.accounts.find((a) => a.code === code);
  if (!account) throw new Error(`seed/branches9.ts: account ${code} missing from the chart of accounts`);
  return account.id;
}

// Starts strictly after the latest timestamp any earlier seed step produced (stock movements,
// journal entries, payments…) so every movement/entry this file adds sorts after everything else
// the seed already posted — `verify:mocks`' "stock movements are in chronological order" check
// depends on insertion order matching timestamp order exactly, and guessing a fixed offset past
// `now` is fragile against however far other seed files' own cursors have already advanced.
// Computed fresh on every `seedBranches9` call (not at module load) since `seedDatabase()` can run
// more than once against a freshly-reset `db` (the dev-menu "reset demo data" action).
let cursor = 0;
function nextTimestamp(now: Date): string {
  cursor = Math.max(cursor, now.getTime()) + 1000;
  return new Date(cursor).toISOString();
}

export function seedBranches9(now: Date, adminId: string, accountantId: string): void {
  // Reset this area's own tables first (matching `seedAccounts`/`seedPeople`/etc.'s convention) —
  // `seedDatabase()` can run more than once against the same `db` object within one browser session
  // (the welcome screen's "استكشف ببيانات تجريبية" card re-seeds without a page reload if clicked
  // again, and so does the dev-menu "reset demo data" action), and this file's `create*` calls
  // validate against `db.branches`/`db.costCenters`/etc. already having unique codes — without this
  // reset, a second seed run collides with the first run's rows instead of replacing them.
  db.branches = [];
  db.costCenters = [];
  db.currencies = [];
  db.exchangeRates = [];
  db.stockTransfers = [];

  cursor = Math.max(
    now.getTime(),
    ...db.stockMovements.map((m) => new Date(m.date).getTime()),
    ...db.journalEntries.map((e) => new Date(e.date).getTime()),
    ...db.payments.map((p) => new Date(p.date).getTime()),
  );
  // --- §1 Branches: the main branch already exists structurally (every pre-phase-9 document
  // implicitly used `DEFAULT_BRANCH_ID`/`branch-main`) — give it a real `Branch` row with that
  // exact id so every already-seeded document/journal-line's `branchId` resolves to something
  // real, then add a second branch so the switcher/branch reports/transfers have something to show.
  db.branches.push({
    id: MAIN_BRANCH_ID,
    name: 'الفرع الرئيسي',
    code: 'RYD',
    address: 'الرياض - حي العليا - شارع التحلية',
    phone: '0112345678',
    cashAccountId: accountId('1110'),
    active: true,
    canDelete: false,
    createdAt: now.toISOString(),
  });
  db.costCenters.push({ id: 'cc-main', code: 'CC-RYD', name: 'الفرع الرئيسي', type: 'branch', active: true, canDelete: false, branchId: MAIN_BRANCH_ID });
  const mainBranch = db.branches[0];
  mainBranch.costCenterId = 'cc-main';

  const jeddah = createBranch(
    { name: 'فرع جدة', code: 'JED', address: 'جدة - حي الروضة - طريق فلسطين', phone: '0122345678', active: true, receiptHeader: 'فرع جدة', defaultPriceListId: undefined },
    adminId,
  );

  // A department cost center too, so the tree isn't only branch-shaped.
  createCostCenter({ name: 'التسويق', code: 'CC-MKT', type: 'department', active: true }, adminId);

  // --- §2 Currencies: USD alongside the SAR base, with a couple of days of rate history. ---------
  createCurrency({ code: 'USD', nameAr: 'دولار أمريكي', symbol: '$', decimals: 2, active: true }, adminId);
  const d1 = new Date(now);
  d1.setDate(d1.getDate() - 3);
  saveExchangeRate({ currency: 'USD', date: d1.toISOString(), rate: 48.5 });
  const d2 = new Date(now);
  d2.setDate(d2.getDate() - 1);
  saveExchangeRate({ currency: 'USD', date: d2.toISOString(), rate: 49.2 });

  // One USD customer (README decision 6: "each customer or supplier has one currency").
  db.customers.push({
    id: 'cus-usd-1',
    code: 'C-0900',
    name: 'Global Trading LLC',
    nameEn: 'Global Trading LLC',
    type: 'company',
    groupId: 'pg-corporate',
    phone: '+12025550111',
    phones: [{ id: 'ph-usd-1', label: 'work', number: '+12025550111' }],
    address: 'Dubai, UAE',
    currency: 'USD',
    paymentTermsDays: 30,
    creditLimit: 5000,
    balance: 0,
    active: true,
  });

  // --- Realized FX worked example (docs/v2/10 §2), pinned exactly by scripts/verify/fx.ts:
  // Invoice USD 1,000 @ 48.50 -> AR 48,500 SAR; receipt USD 1,000 @ 49.20 -> cash 49,200 SAR;
  // Dr Cash(USD) 49,200 / Cr AR[Global Trading] 48,500 / Cr FX gain 700. Party nets to 0 USD.
  // A single free-text service line keeps the FX example's numbers exact and independent of
  // whatever the catalog's prices/stock happen to be (no COGS/inventory noise in the pinned test).
  const revenueAccount = accountId('4110'); // إيرادات الخدمات
  const invoiceDate = nextTimestamp(now);
  const usdInvoice = recordSale(
    {
      customerId: 'cus-usd-1',
      lines: [{ productId: 'freetext-fx-demo', qty: 1, price: 1000, isFreeText: true, revenueAccountId: revenueAccount, name: 'خدمات استشارية' }],
      discountRate: 0,
      paymentMethod: 'credit',
      paidAmount: 0,
      source: 'DESK',
      currency: 'USD',
      exchangeRate: 48.5,
      branchId: MAIN_BRANCH_ID,
    },
    accountantId,
    invoiceDate,
  );

  const paymentDate = nextTimestamp(now);
  recordPayment(
    {
      date: paymentDate,
      type: 'RECEIVED',
      targetType: 'customer',
      targetId: 'cus-usd-1',
      amount: 49200,
      method: 'bank_transfer',
      note: 'تحصيل فاتورة USD',
      currency: 'USD',
      amountFc: 1000,
      rate: 49.2,
      allocations: [{ targetKind: 'invoice', targetId: usdInvoice.id, amount: 49200 }],
      branchId: MAIN_BRANCH_ID,
    },
    accountantId,
  );

  // --- §3 Cost-center split demo: a manual "rent" journal split 60% main / 40% Jeddah. -----------
  const rentAccount = accountId('6220'); // الإيجار
  const cashAccount = accountId('1110');
  const splitLines = splitLineByCostCenters(
    { accountId: rentAccount, description: 'إيجار الشهر — موزع على الفروع', debit: 5000, credit: 0, branchId: MAIN_BRANCH_ID },
    [
      { costCenterId: mainBranch.costCenterId!, pct: 60 },
      { costCenterId: jeddah.costCenterId!, pct: 40 },
    ],
  );
  recordManualJournal(
    {
      date: nextTimestamp(now),
      description: 'إيجار الشهر — موزع على الفروع (مركز تكلفة)',
      lines: [...splitLines, { accountId: cashAccount, description: 'دفع الإيجار', debit: 0, credit: 5000, branchId: MAIN_BRANCH_ID }],
    },
    adminId,
  );

  // --- §4 Stock transfer demo (docs/v2/07-products-and-inventory.md §4): draft -> send -> receive,
  // with a small shortage so the shortage/inventoryVariance path is exercised too, not just the
  // happy path. Picks whichever tracked product has the most stock at the main branch.
  const transferProduct = db.products
    .filter((p) => p.type === 'product' && p.stockMode !== 'none')
    .sort((a, b) => branchStockQty(b.id, MAIN_BRANCH_ID) - branchStockQty(a.id, MAIN_BRANCH_ID))[0];
  if (transferProduct && branchStockQty(transferProduct.id, MAIN_BRANCH_ID) >= 4) {
    const sendQty = Math.min(4, Math.floor(branchStockQty(transferProduct.id, MAIN_BRANCH_ID) / 2));
    const transfer = draftTransfer({ fromBranchId: MAIN_BRANCH_ID, toBranchId: jeddah.id, date: nextTimestamp(now), note: 'تزويد فرع جدة', lines: [{ productId: transferProduct.id, qty: sendQty }] }, adminId);
    sendTransfer(transfer.id, adminId, nextTimestamp(now));
    // Receive one unit short, to demonstrate the shortage -> inventoryVariance posting.
    receiveTransfer(transfer.id, { lines: [{ productId: transferProduct.id, receivedQty: Math.max(0, sendQty - 1) }] }, adminId, nextTimestamp(now));
  }

  // --- Feature switches ON in the demo seed, so all three dimensions are visible by default. ------
  db.settings.features = { branches: true, currencies: true, costCenters: true };
}
