/** v2 (docs/v2/01-personas.md §5, README decision 10): "أمين مخزن" (storekeeper) — inventory write,
 * purchase receiving, transfers, labels; no accounting, no purchase prices visible unless allowed. */
export type Role = 'admin' | 'manager' | 'accountant' | 'cashier' | 'storekeeper';

/** domain_model.md §10 — plus `username` (needed for the mock login screen). */
export interface User {
  id: string;
  username: string;
  name: string;
  phone?: string;
  role: Role;
  maxDiscount: number;
  priceListId?: string;
  active: boolean;
  avatar?: string;
}

export interface UserInput {
  username: string;
  name: string;
  phone?: string;
  role: Role;
  maxDiscount: number;
  priceListId?: string;
  active: boolean;
  /** Only sent when creating a user or changing the password. */
  password?: string;
}

/** Permission areas used for route + nav + action gating (role presets, no permission matrix). */
export type Area =
  | 'dashboard'
  | 'pos'
  | 'sales'
  | 'inventory'
  | 'parties'
  | 'purchases'
  /** v2 phase 8 (docs/v2/01-personas.md §5 role matrix "expenses" row) — its own area: cashier gets
   *  write (pay-out from drawer) despite having no purchases access at all. */
  | 'expenses'
  | 'accounting'
  | 'payments'
  | 'reports'
  | 'users'
  | 'settings';

export type Access = 'none' | 'read' | 'write';
