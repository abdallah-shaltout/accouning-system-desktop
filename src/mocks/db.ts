import type { Account, AccountGroup, FiscalYear, JournalEntry } from '@/modules/accounting/types';
import type { ActivityEntry } from '@/modules/core/types';
import type { Invoice, Refund } from '@/modules/invoices/types';
import type { Customer, Supplier } from '@/modules/parties/types';
import type { Payment } from '@/modules/payments/types';
import type {
  Category,
  PriceList,
  Product,
  StockAdjustment,
  StockMovement,
  Unit,
} from '@/modules/products/types';
import type { PurchaseOrder, PurchaseReturn } from '@/modules/purchases/types';
import type { StoreSettings, Tax } from '@/modules/settings/types';
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
  accountGroups: AccountGroup[];
  accounts: Account[];
  journalEntries: JournalEntry[];
  fiscalYears: FiscalYear[];
  categories: Category[];
  units: Unit[];
  priceLists: PriceList[];
  products: Product[];
  stockAdjustments: StockAdjustment[];
  stockMovements: StockMovement[];
  customers: Customer[];
  suppliers: Supplier[];
  invoices: Invoice[];
  refunds: Refund[];
  purchaseOrders: PurchaseOrder[];
  purchaseReturns: PurchaseReturn[];
  payments: Payment[];
  taxes: Tax[];
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
  | 'adjustment';

export const db: MockDb = {
  users: [],
  credentials: {},
  accountGroups: [],
  accounts: [],
  journalEntries: [],
  fiscalYears: [],
  categories: [],
  units: [],
  priceLists: [],
  products: [],
  stockAdjustments: [],
  stockMovements: [],
  customers: [],
  suppliers: [],
  invoices: [],
  refunds: [],
  purchaseOrders: [],
  purchaseReturns: [],
  payments: [],
  taxes: [],
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
  },
};

const PREFIX: Record<Exclude<DocumentKind, 'invoice'>, string> = {
  refund: 'RET-',
  purchaseOrder: 'PO-',
  purchaseReturn: 'PR-',
  payment: 'PAY-',
  journal: 'JE-',
  adjustment: 'ADJ-',
};

/** Next human-readable document number, e.g. "INV-000457". */
export function nextNumber(kind: DocumentKind): string {
  db.counters[kind] += 1;
  const prefix = kind === 'invoice' ? db.settings.invoiceNumberPrefix : PREFIX[kind];
  return `${prefix}${String(db.counters[kind]).padStart(6, '0')}`;
}

/** Stand-in for a server-side session: the id of the user making mock "requests". */
export const session = { userId: '' };
