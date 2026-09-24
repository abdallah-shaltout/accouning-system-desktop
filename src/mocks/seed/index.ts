/**
 * Seed runner: composes the per-area seed files in the correct order. Each area file only knows
 * its own slice of fixtures — this is the one place that knows the order they must load in
 * (accounts/catalog/people/settings before the opening postings, which need accounts to exist;
 * then the historical replay, which needs everything above).
 *
 * Other Phase-0-parallel tracks and later phases add `src/mocks/seed/<area>.ts` files and wire
 * them in here — never edit the individual area files' internals from an unrelated track.
 */
import { db } from '../db';
import { closeOpeningBalanceEquity } from '../backend/opening';
import { postOpeningCapital, seedAccounts } from './accounts';
import { postOpeningStock, seedCatalog } from './catalog';
import { seedPeople } from './people';
import { seedSettings } from './settings';
import { seedShifts } from './shifts';
import { ADMIN, seedHistory } from './history';
import { seedPurchases8 } from './purchases8';
import { seedBranches9 } from './branches9';

/**
 * Full demo seed: fixtures for every area, the opening position, then a deterministic replay of
 * ~75 days of sales/purchases/payments/refunds/stocktakes. This is what the welcome screen's
 * "explore with demo data" card runs.
 */
export function seedDatabase(now = new Date()): void {
  seedAccounts(now);
  seedCatalog();
  seedPeople();
  seedSettings();

  const day0 = new Date(now);
  day0.setDate(day0.getDate() - 75);
  day0.setHours(8, 30, 0, 0);

  postOpeningCapital(day0.toISOString(), ADMIN);
  const stockDate = new Date(day0);
  stockDate.setMinutes(stockDate.getMinutes() + 30);
  postOpeningStock(stockDate.toISOString(), ADMIN);
  // v2 phase 5 (docs/v2/05-onboarding.md §3 "Closing 3900", invariant 9): `postOpeningStock` above
  // credits 3900 (openingBalanceEquity) for every seeded product's opening quantity — closing it
  // to capital right after is what the real wizard's step 8 does, and is what keeps 3900 at exactly
  // zero once "onboarding" (the demo seed's equivalent) is complete.
  closeOpeningBalanceEquity(stockDate.toISOString(), 'capital', ADMIN);

  seedHistory(now);
  seedShifts(now);
  // v2 phase 8 (docs/v2/09-purchases-payments-expenses.md): purchase v2/landed-cost/expense/voucher/
  // card-settlement demo data. Runs last — the card settlement needs seedHistory's card tenders.
  // usr-2/usr-3 are history.ts's MANAGER/ACCOUNTANT seed users (not exported; same fixture ids).
  seedPurchases8(now, ADMIN, 'usr-3', 'usr-2');
  // v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md): branches, currencies, cost
  // centers demo data. Runs last — needs every other area's fixtures/history already seeded.
  seedBranches9(now, ADMIN, 'usr-3');
}

/**
 * Minimal empty company shell for "start your company" (welcome screen). Sets up an empty chart
 * of accounts placeholder, empty catalog/party lists, sensible default settings and one admin
 * user so the app can reach the login screen without crashing.
 *
 * Phase 5's 11-step onboarding wizard (docs/v2/05-onboarding.md) calls this itself as its own
 * bootstrap step (see `SetupWizardPage.vue`) before walking the owner through CoA template,
 * opening balances and imports — this isn't a stub waiting to be replaced, it's the shell every
 * fresh company starts from either way.
 */
export function seedEmptyCompany(): void {
  seedAccounts(new Date());
  // v2 phase 9: every document/journal-line implicitly uses `DEFAULT_BRANCH_ID`/`branch-main` (see
  // src/mocks/backend/core.ts) even before the owner ever opens Settings → Branches, so a real
  // `Branch` row with that exact id must exist from the very first posting, not only once the
  // feature switch is turned on — otherwise branch reports/pickers see a dangling id.
  const mainCash = db.accounts.find((a) => a.code === '1110');
  db.branches = [{ id: 'branch-main', name: 'الفرع الرئيسي', code: 'MAIN', cashAccountId: mainCash?.id, costCenterId: 'cc-main', active: true, canDelete: false, createdAt: new Date().toISOString() }];
  db.costCenters = [{ id: 'cc-main', code: 'CC-MAIN', name: 'الفرع الرئيسي', type: 'branch', active: true, canDelete: false, branchId: 'branch-main' }];
  db.currencies = [];
  db.exchangeRates = [];
  db.stockTransfers = [];
  db.categories = [];
  db.units = [];
  db.priceLists = [];
  db.products = [];
  db.customers = [];
  db.suppliers = [];
  // v2 phase 3: category/direction are required on Tax now (docs/v2/06-sales-and-pos.md §3) — kept
  // inline here rather than importing the demo fixture, since this is the *empty* company shell.
  db.taxes = [
    { id: 'tax-vat-out', name: 'ضريبة القيمة المضافة (مبيعات)', rate: 15, type: 'OUTPUT', isDefault: true, active: true, category: 'S', direction: 'sales', accountRole: 'vatOutput' },
    { id: 'tax-vat-in', name: 'ضريبة القيمة المضافة (مشتريات)', rate: 15, type: 'INPUT', isDefault: true, active: true, category: 'S', direction: 'purchase', accountRole: 'vatInput' },
  ];
  db.paymentMethods = [
    { id: 'pm-cash', name: 'نقداً', type: 'cash', accountRole: 'cash', feePct: 0, showInPos: true, showInPayments: true, sortOrder: 1, active: true, canDelete: false },
    { id: 'pm-credit', name: 'آجل', type: 'credit', accountRole: 'receivable', feePct: 0, showInPos: true, showInPayments: false, sortOrder: 2, active: true, canDelete: false },
  ];
  db.settings = {
    storeName: 'شركتي',
    currency: 'SAR',
    defaultTaxId: 'tax-vat-out',
    invoiceNumberPrefix: 'INV-',
    printer: { mode: 'a4', thermalWidthMm: 80 },
    theme: 'light',
    pricesIncludeTax: true,
  };
  db.users = [{ id: 'usr-1', username: 'admin', name: 'المدير', role: 'admin', maxDiscount: 100, active: true }];
  db.credentials = { admin: 'admin123' };
}
