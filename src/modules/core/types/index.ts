export type ActivityKind =
  | 'sale'
  | 'refund'
  | 'purchase'
  | 'purchase_return'
  | 'payment'
  | 'stock'
  | 'journal'
  | 'product'
  | 'party'
  | 'user'
  | 'settings'
  | 'auth'
  | 'shift'
  /** v2 phase 8 (docs/v2/09-purchases-payments-expenses.md §4-§5): expenses and general vouchers. */
  | 'expense'
  | 'voucher';

export interface ActivityEntry {
  id: string;
  date: string;
  userId: string;
  kind: ActivityKind;
  message: string;
  /** Route to open when the entry is clicked, e.g. `/invoices/inv-12`. */
  link?: string;
}

export interface DateRange {
  from: string;
  to: string;
}

export interface DashboardSummary {
  todaySales: number;
  todayInvoiceCount: number;
  unpaidInvoiceCount: number;
  unpaidInvoiceTotal: number;
  lowStockCount: number;
  cashPosition: number;
  cashOnHand: number;
  bankBalance: number;
  /** Last 14 days of net sales, oldest first. */
  salesTrend: { date: string; total: number }[];
}
