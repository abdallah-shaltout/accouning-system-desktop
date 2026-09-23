export type NormalSide = 'DEBIT' | 'CREDIT';

export type AccountGroupKind = 'ASSET' | 'LIABILITY' | 'EQUITY' | 'REVENUE' | 'EXPENSE';

export interface AccountGroup {
  id: string;
  code: string;
  name: string;
  /** Extension: which financial-statement section this root group belongs to. */
  kind: AccountGroupKind;
  normalSide: NormalSide;
  isDefault: boolean;
}

export interface Account {
  id: string;
  /** Business key, e.g. "1110" — the only identifying code (no internalCode). */
  code: string;
  name: string;
  groupId: string;
  parentId?: string;
  normalSide: NormalSide;
  canDelete: boolean;
  active: boolean;
}

export interface AccountInput {
  code: string;
  name: string;
  groupId: string;
  parentId?: string;
  normalSide: NormalSide;
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
  lines: { accountId: string; description?: string; debit: number; credit: number }[];
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
