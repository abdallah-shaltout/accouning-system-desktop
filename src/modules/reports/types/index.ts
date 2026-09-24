export interface DateRangeInput {
  from?: string;
  to?: string;
}

export interface TrialBalanceRow {
  accountId: string;
  code: string;
  name: string;
  groupName: string;
  openingBalance: number; // signed: + debit / − credit
  periodDebit: number;
  periodCredit: number;
  closingDebit: number;
  closingCredit: number;
}

export interface StatementLine {
  accountId: string;
  code: string;
  name: string;
  amount: number;
}

export interface ProfitAndLoss {
  revenue: StatementLine[];
  netRevenue: number;
  cogs: StatementLine[];
  totalCogs: number;
  grossProfit: number;
  expenses: StatementLine[];
  totalExpenses: number;
  netIncome: number;
}

/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §3 "P&L by cost center: a column per
 * center, with a drill-down"). `centers` is every active cost center that had any movement in the
 * range, plus a synthetic `unassigned` column for lines with no `costCenterId` at all (pre-phase-9
 * data, or accounts that don't require one) — so Σ columns always equals the flat P&L's totals.
 */
export interface CostCenterPnlColumn {
  costCenterId: string;
  name: string;
  netRevenue: number;
  totalCogs: number;
  grossProfit: number;
  totalExpenses: number;
  netIncome: number;
}

export interface CostCenterPnl {
  centers: CostCenterPnlColumn[];
  unassigned: CostCenterPnlColumn;
  total: CostCenterPnlColumn;
}

/** Budget vs actual for one cost center in one fiscal year (docs/v2/10 §3). */
export interface CostCenterBudgetRow {
  costCenterId: string;
  name: string;
  budget: number;
  actual: number;
  variancePct: number;
  /** True once actual crosses 90% of budget — the insight stub this phase leaves for Phase 10. */
  nearBudget: boolean;
}

export interface BalanceSheet {
  asOf: string;
  assets: StatementLine[];
  totalAssets: number;
  liabilities: StatementLine[];
  totalLiabilities: number;
  equity: StatementLine[];
  /** Revenue − expenses to date that hasn't been closed into retained earnings. */
  unclosedEarnings: number;
  totalEquity: number;
  balanced: boolean;
}

export interface LedgerRow {
  id: string;
  date: string;
  entryId: string;
  entryNumber: string;
  description: string;
  debit: number;
  credit: number;
  balance: number;
}

export interface AccountLedger {
  title: string;
  subtitle?: string;
  normalSide: 'DEBIT' | 'CREDIT';
  openingBalance: number;
  rows: LedgerRow[];
  totalDebit: number;
  totalCredit: number;
  closingBalance: number;
  /** Where each row links: journal entries (GL) or source documents (party statements). */
  rowLinks: Record<string, string>;
}

export interface SalesReport {
  summary: {
    invoiceCount: number;
    grossSales: number;
    discounts: number;
    netSales: number;
    vat: number;
    total: number;
    refunds: number;
    netAfterRefunds: number;
    cogs: number;
    grossProfit: number;
    averageInvoice: number;
  };
  byDay: { date: string; invoices: number; total: number }[];
  byProduct: { productId: string; name: string; qty: number; revenue: number; cost: number; profit: number }[];
  byCategory: { name: string; qty: number; revenue: number }[];
  byMethod: { method: string; count: number; total: number }[];
  byCashier: { name: string; count: number; total: number }[];
}

export interface InventoryReportRow {
  productId: string;
  name: string;
  sku: string;
  category: string;
  qty: number;
  minStock: number;
  costPrice: number;
  price: number;
  costValue: number;
  retailValue: number;
  status: 'ok' | 'low' | 'out';
}

/** v2 (docs/v2/06-sales-and-pos.md §3 step 5, docs/v2/02-accounting-review.md D1): one VAT-return box per (category, rate). */
export interface VatCategoryBox {
  category: 'S' | 'Z' | 'E' | 'O';
  rate: number;
  net: number;
  vat: number;
  count: number;
}

export interface VatReport {
  sales: { taxable: number; vat: number; count: number };
  salesReturns: { taxable: number; vat: number; count: number };
  purchases: { taxable: number; vat: number; count: number };
  purchaseReturns: { taxable: number; vat: number; count: number };
  outputVat: number;
  inputVat: number;
  netPayable: number;
  /** Cross-check against the VAT accounts in the ledger. */
  ledgerOutput: number;
  ledgerInput: number;
  /**
   * v2: per-line sales VAT grouped by (category, rate) — docs/v2/02-accounting-review.md D1's
   * "different VAT-return boxes" for zero-rated vs exempt. Computed from `invoice.lines[].taxCategory`
   * net of the same lines' share of credit notes, so `Σ salesBoxes[].vat = outputVat` exactly.
   */
  salesBoxes: VatCategoryBox[];
}

// ---------------------------------------------------------------------------------------------
// v2 phase 12 (docs/v2/13-reports.md) — ReportShell v2 filter/comparison shape + new report types.
// ---------------------------------------------------------------------------------------------

/** Dimension filters a report query can accept — each is applied only when the value is set. */
export interface DimensionFilter {
  branchId?: string;
  costCenterId?: string;
  currency?: string;
}

export type ReportRangeFilter = DateRangeInput & DimensionFilter;

/** `ReportShell`'s comparison mode (docs/v2/13-reports.md §1 "comparison"). */
export type ComparisonMode = 'none' | 'previousPeriod' | 'sameLastYear';

// --- Cash flow (indirect method) ----------------------------------------------------------------

export interface CashFlowLine {
  label: string;
  amount: number;
}

export interface CashFlowStatement {
  netIncome: number;
  operatingAdjustments: CashFlowLine[];
  operatingCash: number;
  investing: CashFlowLine[];
  investingCash: number;
  financing: CashFlowLine[];
  financingCash: number;
  netChange: number;
  openingCash: number;
  closingCash: number;
}

// --- Day book -------------------------------------------------------------------------------

export interface DayBookLine {
  accountCode: string;
  accountName: string;
  debit: number;
  credit: number;
}

export interface DayBookEntry {
  id: string;
  number: string;
  date: string;
  description: string;
  lines: DayBookLine[];
}

// --- AR / AP aging ---------------------------------------------------------------------------

export interface AgingReportRow {
  partyId: string;
  name: string;
  current: number;
  b30: number;
  b60: number;
  b90plus: number;
  total: number;
}

// --- Overdue invoices ------------------------------------------------------------------------

export interface OverdueRow {
  id: string;
  kind: 'invoice' | 'purchaseOrder';
  number: string;
  partyId: string;
  partyName: string;
  phone?: string;
  date: string;
  dueDate?: string;
  daysOverdue: number;
  outstanding: number;
}

// --- Gross profit ---------------------------------------------------------------------------

export interface GrossProfitRow {
  key: string;
  label: string;
  qty: number;
  revenue: number;
  cost: number;
  profit: number;
  marginPct: number;
}

// --- Returns analysis ------------------------------------------------------------------------

export interface ReturnsReportRow {
  key: string;
  label: string;
  count: number;
  qty: number;
  amount: number;
}

export interface ReturnsReport {
  totalInvoices: number;
  totalRefunds: number;
  returnRatePct: number;
  byReason: ReturnsReportRow[];
  byProduct: ReturnsReportRow[];
  byCashier: ReturnsReportRow[];
}

// --- Discounts & price overrides --------------------------------------------------------------

export interface DiscountReportRow {
  key: string;
  label: string;
  invoiceCount: number;
  listValue: number;
  chargedValue: number;
  discountValue: number;
  discountPct: number;
}

// --- Shifts / Z-report history -----------------------------------------------------------------

export interface ShiftReportRow {
  id: string;
  number: string;
  terminalId: string;
  openedAt: string;
  closedAt?: string;
  openedBy: string;
  closedBy?: string;
  expectedCash: number;
  countedCash: number;
  variance: number;
  salesTotal: number;
}

// --- Low / dead stock ------------------------------------------------------------------------

export interface LowStockRow {
  productId: string;
  name: string;
  sku: string;
  category: string;
  qty: number;
  minStock: number;
  suggestedQty: number;
  costValue: number;
}

export interface DeadStockRow {
  productId: string;
  name: string;
  sku: string;
  category: string;
  qty: number;
  costValue: number;
  lastSaleDate?: string;
  daysSinceSale: number;
}

// --- Stocktake variances ---------------------------------------------------------------------

export interface StocktakeVarianceRow {
  countId: string;
  countNumber: string;
  productId: string;
  name: string;
  category: string;
  countedQty: number;
  systemQty: number;
  qtyVariance: number;
  valueVariance: number;
}

// --- Transfers ------------------------------------------------------------------------------

export interface TransferReportRow {
  id: string;
  number: string;
  date: string;
  fromBranch: string;
  toBranch: string;
  status: string;
  sentQty: number;
  receivedQty: number;
  shortageQty: number;
  shortageValue: number;
}

// --- Purchases summary -----------------------------------------------------------------------

export interface PurchasesReport {
  summary: { poCount: number; grossPurchases: number; vat: number; total: number; returns: number };
  bySupplier: { supplierId: string; name: string; count: number; total: number }[];
  byProduct: { productId: string; name: string; qty: number; total: number; avgPrice: number }[];
}

// --- Expenses report -------------------------------------------------------------------------

export interface ExpensesReport {
  total: number;
  byCategory: { categoryId: string; name: string; amount: number }[];
  byMonth: { month: string; amount: number }[];
}

// --- Period / branch comparison ----------------------------------------------------------------

export interface PeriodComparisonLine {
  label: string;
  a: number;
  b: number;
  delta: number;
  deltaPct: number;
}

export interface BranchComparisonRow {
  branchId: string;
  name: string;
  sales: number;
  grossProfit: number;
  invoiceCount: number;
  averageInvoice: number;
}

// --- Business health ------------------------------------------------------------------------

export interface BusinessHealthScore {
  key: 'liquidity' | 'profitability' | 'debt' | 'collection';
  label: string;
  score: number; // 0-25
  value: number; // raw ratio/metric
  explanation: string;
}

export interface BusinessHealthReport {
  total: number; // 0-100
  scores: BusinessHealthScore[];
}

// --- VAT detail -----------------------------------------------------------------------------

/** v2 phase 12 (docs/v2/13 §2 "VAT detail: line-level listing per document with category, net, VAT — the audit trail behind each box"). */
export interface VatDetailRow {
  id: string;
  date: string;
  documentNumber: string;
  documentKind: 'sale' | 'salesReturn';
  productName: string;
  category: 'S' | 'Z' | 'E' | 'O';
  rate: number;
  net: number;
  vat: number;
}

// --- Profit leakage --------------------------------------------------------------------------

export interface ProfitLeakageReport {
  netSales: number;
  discounts: number;
  returns: number;
  writeOffs: number;
  shrinkage: number;
  totalLeakage: number;
  leakagePct: number;
}
