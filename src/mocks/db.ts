import type { Account, FiscalYear, JournalEntry, JournalTemplate } from '@/modules/accounting/types';
import type { ActivityEntry } from '@/modules/core/types';
import type { Invoice, Refund } from '@/modules/invoices/types';
import type { Customer, PartyGroup, PartyHistoryEntry, Supplier } from '@/modules/parties/types';
import type { Payment } from '@/modules/payments/types';
import type {
  Category,
  CustomFieldDef,
  DebitNoteDraft,
  PriceList,
  Product,
  ProductBatch,
  StockAdjustment,
  StockCount,
  StockMovement,
  Unit,
} from '@/modules/products/types';
import type { PurchaseOrder, PurchaseReturn } from '@/modules/purchases/types';
import type { PaymentMethod, StoreSettings, Tax } from '@/modules/settings/types';
import type { User } from '@/modules/users/types';

/**
 * The in-memory "database" behind every mock service. It lives only for the lifetime of the
 * page — reloading the app re-seeds it (see seed.ts). Replace the services, not the pages,
 * when a real backend arrives.
 */
export interface MockDb {
  users: User[];
  /** username → password. Plain text on purpose: this is a UI mock, not an auth system. */
  credentials: Record<string, string>;
  accounts: Account[];
  journalEntries: JournalEntry[];
  /**
   * v2 phase 2 (docs/v2/11-journal-dashboard-insights.md A2): entries saved without posting.
   * Kept OUT of `journalEntries` on purpose — every ledger/balance/report reader in the app scans
   * `journalEntries` expecting only posted GL impact (party balances, trial balance, verify:mocks
   * invariants…), and none of those readers are this phase's to edit. A draft becomes a real
   * `journalEntries` row only once `postDraftJournal` posts it.
   */
  journalDrafts: JournalEntry[];
  journalTemplates: JournalTemplate[];
  fiscalYears: FiscalYear[];
  categories: Category[];
  units: Unit[];
  priceLists: PriceList[];
  products: Product[];
  stockAdjustments: StockAdjustment[];
  stockMovements: StockMovement[];
  /** v2 phase 6 (docs/v2/07-products-and-inventory.md §3): batches/lots for batch-tracked products. */
  productBatches: ProductBatch[];
  /** v2 phase 6 (Settings → Products): custom field definitions shown on the product form's "إضافي" tab. */
  customFieldDefs: CustomFieldDef[];
  /** v2 phase 6 (§5 stocktake v2): scope + snapshot + blind/scan counting sessions, applied into a STOCKTAKE StockAdjustment. */
  stockCounts: StockCount[];
  /** v2 phase 6 (§4 expiry report "return to supplier"). TODO(phase 8): superseded by real debit notes. */
  debitNoteDrafts: DebitNoteDraft[];
  customers: Customer[];
  suppliers: Supplier[];
  /** v2 phase 4 (docs/v2/08-customers-and-suppliers.md §5): Settings → Parties groups. */
  partyGroups: PartyGroup[];
  /** v2 phase 4 (docs/v2/08 §3 "السجل"): per-party audit trail, separate from the shared `activity` feed. */
  partyHistory: PartyHistoryEntry[];
  invoices: Invoice[];
  refunds: Refund[];
  purchaseOrders: PurchaseOrder[];
  purchaseReturns: PurchaseReturn[];
  payments: Payment[];
  taxes: Tax[];
  /** v2 phase 3 (docs/v2/09-purchases-payments-expenses.md §2): payment methods → settlement account by role. */
  paymentMethods: PaymentMethod[];
  settings: StoreSettings;
  activity: ActivityEntry[];
  counters: Record<DocumentKind, number>;
}

export type DocumentKind =
  | 'invoice'
  | 'refund'
  | 'purchaseOrder'
  | 'purchaseReturn'
  | 'payment'
  | 'journal'
  | 'adjustment'
  | 'stockCount'
  | 'debitNoteDraft';

export const db: MockDb = {
  users: [],
  credentials: {},
  accounts: [],
  journalEntries: [],
  journalDrafts: [],
  journalTemplates: [],
  fiscalYears: [],
  categories: [],
  units: [],
  priceLists: [],
  products: [],
  stockAdjustments: [],
  stockMovements: [],
  productBatches: [],
  customFieldDefs: [],
  stockCounts: [],
  debitNoteDrafts: [],
  customers: [],
  suppliers: [],
  partyGroups: [],
  partyHistory: [],
  invoices: [],
  refunds: [],
  purchaseOrders: [],
  purchaseReturns: [],
  payments: [],
  taxes: [],
  paymentMethods: [],
  settings: {
    storeName: '',
    currency: 'SAR',
    invoiceNumberPrefix: 'INV-',
    printer: { mode: 'a4', thermalWidthMm: 80 },
    theme: 'light',
  },
  activity: [],
  counters: {
    invoice: 0,
    refund: 0,
    purchaseOrder: 0,
    purchaseReturn: 0,
    payment: 0,
    journal: 0,
    adjustment: 0,
    stockCount: 0,
    debitNoteDraft: 0,
  },
};

const PREFIX: Record<Exclude<DocumentKind, 'invoice'>, string> = {
  refund: 'RET-',
  purchaseOrder: 'PO-',
  purchaseReturn: 'PR-',
  payment: 'PAY-',
  journal: 'JE-',
  adjustment: 'ADJ-',
  stockCount: 'CNT-',
  debitNoteDraft: 'DN-',
};

/** Next human-readable document number, e.g. "INV-000457". */
export function nextNumber(kind: DocumentKind): string {
  db.counters[kind] += 1;
  const prefix = kind === 'invoice' ? db.settings.invoiceNumberPrefix : PREFIX[kind];
  return `${prefix}${String(db.counters[kind]).padStart(6, '0')}`;
}

/** Stand-in for a server-side session: the id of the user making mock "requests". */
export const session = { userId: '' };
