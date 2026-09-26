/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md): branches, cost centers and
 * currencies — the three "dimensions" that were reserved as inert fields on journal lines and
 * documents since Phase 1 (`branchId`, `costCenterId`, `currency`/`amountFc`/`rate`). This file
 * defines the entities Settings manages; the fields on `JournalLine`/documents stay in
 * `modules/accounting/types` etc. — only the master records live here.
 */

// ---------------------------------------------------------------------------------------------
// Branches (§1)
// ---------------------------------------------------------------------------------------------

export interface Branch {
  id: string;
  name: string;
  /** Business key used as the numbering-series prefix, e.g. "RYD" → "RYD-INV-00042". */
  code: string;
  address?: string;
  /** doc 18.E: structured address (region/city/district picker + street/building). Preferred over the legacy `address` string when present. */
  nationalAddress?: import('@/modules/core/types/address').Address;
  phone?: string;
  /** Printed at the top of receipts issued from this branch. */
  receiptHeader?: string;
  /** Auto-created cash-drawer account id ("111x الصندوق — <branch>"), via accountFor-style resolution. */
  cashAccountId?: string;
  /** Default bank account id for this branch's bank tenders/deposits. */
  bankAccountId?: string;
  defaultPriceListId?: string;
  /** The branch's own cost center (auto-created with the branch, docs/v2/10 §3). */
  costCenterId?: string;
  active: boolean;
  /** The first branch (seed's main branch) can't be deleted, only deactivated. */
  canDelete: boolean;
  createdAt?: string;
}

export type BranchInput = Omit<Branch, 'id' | 'cashAccountId' | 'costCenterId' | 'canDelete' | 'createdAt'>;

// ---------------------------------------------------------------------------------------------
// Cost centers (§3)
// ---------------------------------------------------------------------------------------------

export type CostCenterType = 'branch' | 'department' | 'project' | 'other';

export interface CostCenterBudget {
  fiscalYearId: string;
  amount: number;
}

export interface CostCenter {
  id: string;
  code: string;
  name: string;
  type: CostCenterType;
  parentId?: string;
  managerUserId?: string;
  active: boolean;
  budgets?: CostCenterBudget[];
  /** Branch cost centers are created with their branch and can't be deleted (docs/v2/10 §3). */
  canDelete: boolean;
  /** Set only on the auto-created branch cost center — links back to its branch. */
  branchId?: string;
}

export type CostCenterInput = Omit<CostCenter, 'id' | 'canDelete' | 'branchId'>;

// ---------------------------------------------------------------------------------------------
// Currencies (§2)
// ---------------------------------------------------------------------------------------------

export interface Currency {
  /** ISO code, e.g. "USD". The base currency's code lives at `StoreSettings.currency`. */
  code: string;
  nameAr: string;
  symbol: string;
  decimals: number;
  active: boolean;
  /** Fixed-rate pegged currency (e.g. SAR/USD) — `fixedRate` is base-per-1-unit, never looked up from the rate table. */
  fixed?: boolean;
  fixedRate?: number;
}

export interface ExchangeRate {
  id: string;
  currency: string;
  date: string;
  /** Base currency per 1 unit of `currency` — the same convention as everywhere else in this phase (`rate` on a journal line, `Account.currency` balances, etc). */
  rate: number;
}

/** Convenience input shape for the rate-entry form, which accepts either direction. */
export interface ExchangeRateInput {
  currency: string;
  date: string;
  /** "1 USD = 3.75 SAR" style: rate is base-per-unit. */
  rate?: number;
  /** "1 SAR = 0.2667 USD" style: inverse is unit-per-base; the form inverts it before saving. */
  inverseRate?: number;
}
