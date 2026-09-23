export type NormalSide = 'DEBIT' | 'CREDIT';

export type AccountKind = 'ASSET' | 'LIABILITY' | 'EQUITY' | 'REVENUE' | 'EXPENSE';

/** Statement classification — drives which report section an account rolls into (03-chart-of-accounts.md §3). */
export type AccountSubtype =
  | 'cash' | 'bank' | 'clearing'
  | 'receivable' | 'payable'
  | 'inventory' | 'tax' | 'prepaid' | 'otherCurrentAsset'
  | 'fixedAsset' | 'accumulatedDepreciation'
  | 'currentLiability' | 'longTermLiability'
  | 'equity'
  | 'revenue' | 'otherIncome'
  | 'costOfSales'
  | 'operatingExpense' | 'otherExpense' | 'zakatTax';

/**
 * System roles — how posting rules find accounts (docs/v2/03-chart-of-accounts.md §2). At most one
 * account per role, except the ones marked with a trailing `*` in the doc (cash/bank), where several
 * accounts may share the role and callers pick by branch/currency (inert until Phase 9 — this phase
 * always resolves to the single seeded account per role).
 */
export type SystemRole =
  | 'cash' | 'bank' | 'cardClearing' | 'walletClearing' | 'receivable' | 'inventory'
  | 'inventoryInTransit' | 'vatInput' | 'payable' | 'vatOutput' | 'vatPayable' | 'customerAdvances'
  | 'capital' | 'ownerCurrent' | 'drawings' | 'retainedEarnings' | 'currentEarnings' | 'openingBalanceEquity'
  | 'sales' | 'serviceRevenue' | 'salesReturns' | 'otherIncome' | 'fxGain' | 'cashOver'
  | 'purchaseDiscounts' | 'cogs' | 'inventoryVariance' | 'inventoryWriteOff' | 'freightIn'
  | 'cardFees' | 'bankFees' | 'fxLoss' | 'cashShort' | 'badDebt' | 'depreciation' | 'zakat';

/**
 * v2 Account: a tree of header (`isGroup`) and postable (leaf) accounts. Posting rules resolve
 * accounts by `systemRole` via `accountFor()`, never by hard-coded `code` — see
 * src/mocks/backend/accounts.ts. `AccountGroup` (v1's flat 5-group root) is gone: the five kinds
 * are just the top-level header accounts (code "1".."5") in this same table.
 */
export interface Account {
  id: string;
  /** Business key, e.g. "1130". Children must start with their parent's code. */
  code: string;
  name: string;
  nameEn?: string;
  parentId: string | null;
  /** Header account: not postable; its balance/rolled balance comes from its children. */
  isGroup: boolean;
  kind: AccountKind;
  subtype: AccountSubtype;
  normalSide: NormalSide;
  systemRole?: SystemRole;
  /** Foreign-currency cash/bank account; undefined/null = base currency. Inert until Phase 9. */
  currency?: string;
  /** Branch-scoped cash drawer etc. Inert until Phase 9 (single default branch for now). */
  branchId?: string;
  /** AR/AP control accounts: a journal line on this account needs a party. */
  requiresParty?: boolean;
  /** false for inventory, VAT in/out, and other accounts only the system may post to. */
  allowManual: boolean;
  /** Optional per expense account. Inert until Phase 9 (no cost-center picker yet). */
  requiresCostCenter?: boolean;
  active: boolean;
  /** System-role accounts can't be deleted, only renamed/renumbered (and not even renumbered here). */
  canDelete: boolean;
}

export interface AccountInput {
  code: string;
  name: string;
  nameEn?: string;
  parentId?: string;
  isGroup: boolean;
  kind: AccountKind;
  subtype: AccountSubtype;
  normalSide: NormalSide;
  requiresParty?: boolean;
  allowManual: boolean;
  active: boolean;
}

export type JournalSourceKind =
  | 'invoice'
  | 'refund'
  | 'purchaseOrder'
  | 'purchaseReturn'
  | 'payment'
  | 'stockAdjustment'
  /** v2 phase 8 (docs/v2/09-purchases-payments-expenses.md §4): expense voucher. */
  | 'expense'
  /** v2 phase 8 (§5): general receipt / general payment / transfer / owner drawings-contribution. */
  | 'voucher'
  /** v2 phase 8 (§2): card/wallet settlement voucher. */
  | 'settlement'
  /** v2 phase 7 (docs/v2/06-sales-and-pos.md §5): shift-close cash variance / cash-drop postings. */
  | 'shift'
  /** v2 phase 9 (docs/v2/10 §2 "Unrealized FX"): the currency-revaluation wizard's auto-reversing entry. */
  | 'fxReval';

export interface JournalLine {
  id: string;
  accountId: string;
  description?: string;
  debit: number;
  credit: number;
  /** Party dimension (review C2) — required on requiresParty accounts, used to compute balances from the ledger. */
  partyKind?: 'customer' | 'supplier';
  partyId?: string;
  /** Dimensions reserved for Phase 9 (branches / cost centers / multi-currency). Always the single
   *  default branch and base currency for now, but populated so no later migration is needed. */
  branchId?: string;
  costCenterId?: string;
  currency?: string;
  amountFc?: number;
  rate?: number;
}

/**
 * v2 (docs/v2/11-journal-dashboard-insights.md Part A, 04-domain-model.md §8). `MANUAL_*` /
 * `OPENING` / `CLOSING` / `VAT_SETTLEMENT` are still posted through the same `postJournal()`
 * choke point as `SYSTEM`; `type` is only a label for the list/filters. `status` defaults to
 * `POSTED` everywhere except drafts saved from the entry form (phase 2 adds draft support —
 * phase 1's `recordManualJournal` never produced anything but a posted entry).
 */
export type JournalEntryType = 'SYSTEM' | 'MANUAL' | 'OPENING' | 'CLOSING' | 'VAT_SETTLEMENT';
export type JournalEntryStatus = 'DRAFT' | 'POSTED';

export interface JournalEntry {
  id: string;
  number: string;
  date: string;
  description: string;
  type: JournalEntryType;
  status: JournalEntryStatus;
  sourceRef?: { kind: JournalSourceKind; id: string; number?: string };
  lines: JournalLine[];
  totalDebit: number;
  totalCredit: number;
  reversed?: boolean;
  reversalOfId?: string;
  /** B3: reversal date + required reason, stored on the reversal entry itself. */
  reversalReason?: string;
  createdBy: string;
  createdAt: string;
  postedBy?: string;
  postedAt?: string;
  attachmentIds?: string[];
  templateId?: string;
}

export interface JournalEntryInput {
  date: string;
  description: string;
  reference?: string;
  lines: {
    accountId: string;
    description?: string;
    debit: number;
    credit: number;
    partyKind?: 'customer' | 'supplier';
    partyId?: string;
    branchId?: string;
    costCenterId?: string;
  }[];
  attachmentIds?: string[];
  /** Save without posting (drafts-to-post view). */
  asDraft?: boolean;
  templateId?: string;
}

export interface JournalFilter {
  type?: JournalEntryType;
  from?: string;
  to?: string;
  search?: string;
  accountId?: string;
  partyId?: string;
  status?: JournalEntryStatus;
  hasAttachments?: boolean;
  reversed?: boolean;
  userId?: string;
  minAmount?: number;
  maxAmount?: number;
  sourceKind?: JournalSourceKind;
}

export interface FiscalYear {
  id: string;
  name: string;
  startDate: string;
  endDate: string;
  isClosed: boolean;
  closingEntryId?: string;
  closedAt?: string;
  closedBy?: string;
}

/** A2/A3 — saved journal template (rent/salaries/depreciation…) and, optionally, its recurrence. */
export interface JournalTemplate {
  id: string;
  name: string;
  description: string;
  lines: {
    accountId: string;
    description?: string;
    debit: number;
    credit: number;
    partyKind?: 'customer' | 'supplier';
    partyId?: string;
  }[];
  recurrence?: {
    every: 'month' | 'quarter' | 'year';
    /** Day of month the entry falls due. */
    day: number;
    nextDate: string;
    autoPost: boolean;
  };
  createdAt: string;
  createdBy: string;
}

/** A1 — per-user saved filter/column preset for the journal list, stored in localStorage. */
export interface JournalSavedView {
  id: string;
  name: string;
  filter: JournalFilter;
}
