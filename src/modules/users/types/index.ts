export type Role = 'admin' | 'manager' | 'accountant' | 'cashier';

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
  | 'accounting'
  | 'payments'
  | 'reports'
  | 'users'
  | 'settings';

export type Access = 'none' | 'read' | 'write';
