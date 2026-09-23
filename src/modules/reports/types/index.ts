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
