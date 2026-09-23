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
  | 'stockAdjustment';

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

export interface JournalEntry {
  id: string;
  number: string;
  date: string;
  description: string;
  type: 'SYSTEM' | 'MANUAL';
  sourceRef?: { kind: JournalSourceKind; id: string; number?: string };
  lines: JournalLine[];
  totalDebit: number;
  totalCredit: number;
  reversed?: boolean;
  reversalOfId?: string;
  createdBy: string;
}

export interface JournalEntryInput {
  date: string;
  description: string;
  lines: { accountId: string; description?: string; debit: number; credit: number; partyKind?: 'customer' | 'supplier'; partyId?: string }[];
}

export interface JournalFilter {
  type?: 'SYSTEM' | 'MANUAL';
  from?: string;
  to?: string;
  search?: string;
  accountId?: string;
}

export interface FiscalYear {
  id: string;
  name: string;
  startDate: string;
  endDate: string;
  isClosed: boolean;
}
