import type { Access, Area, Role } from '../types';

/**
 * Role presets (domain_model.md §10) — replaces the reference system's 100+ permission strings.
 *   admin       everything
 *   manager     everything except user management
 *   accountant  accounting, reports, payments, sales (write, per docs/v2/01-personas.md §3 "Full
 *               invoice form" + the role-matrix table); purchases/inventory/parties read-only
 *   cashier     POS/sales only, read-only inventory & customers, no accounting/reports
 *   storekeeper v2 (docs/v2/01-personas.md §5, README decision 10): inventory + receiving write,
 *               read-only catalog (except units/barcodes — see `roleCanEditUnits`), no accounting/
 *               payments/analytics. `dashboard: 'read'` gets them their stock-panel home; POS stays
 *               `none` (selling isn't their job, per the role matrix table).
 */
export const ROLE_ACCESS: Record<Role, Record<Area, Access>> = {
  admin: {
    dashboard: 'write', pos: 'write', sales: 'write', inventory: 'write', parties: 'write',
    purchases: 'write', expenses: 'write', accounting: 'write', payments: 'write', reports: 'write', analytics: 'write', approvals: 'write', users: 'write', settings: 'write',
  },
  manager: {
    dashboard: 'write', pos: 'write', sales: 'write', inventory: 'write', parties: 'write',
    purchases: 'write', expenses: 'write', accounting: 'write', payments: 'write', reports: 'write', analytics: 'write', approvals: 'write', users: 'none', settings: 'write',
  },
  accountant: {
    dashboard: 'read', pos: 'none', sales: 'write', inventory: 'read', parties: 'read',
    purchases: 'read', expenses: 'write', accounting: 'write', payments: 'write', reports: 'write', analytics: 'read', approvals: 'none', users: 'none', settings: 'none',
  },
  cashier: {
    dashboard: 'read', pos: 'write', sales: 'write', inventory: 'read', parties: 'read',
    purchases: 'none', expenses: 'write', accounting: 'none', payments: 'none', reports: 'none', analytics: 'none', approvals: 'none', users: 'none', settings: 'none',
  },
  storekeeper: {
    dashboard: 'read', pos: 'none', sales: 'none', inventory: 'write', parties: 'read',
    purchases: 'write', expenses: 'none', accounting: 'none', payments: 'none', reports: 'read', analytics: 'none', approvals: 'none', users: 'none', settings: 'none',
  },
};

const RANK: Record<Access, number> = { none: 0, read: 1, write: 2 };

/**
 * v2 (docs/v2/07-products-and-inventory.md §6 "Role matrix editor"): `Settings → Users & roles`
 * lets an admin adjust a preset's area access on top of `ROLE_ACCESS`. Overrides are sparse — only
 * the (role, area) cells an admin actually changed — and live in `db.settings.roleAccessOverrides`
 * so they persist and survive a reload like any other setting. `roleCan` below is the only reader;
 * everything else (route guards, sidebar, `auth.can()`) keeps calling `roleCan`/`auth.can()`
 * unchanged, so the override layer is invisible to the rest of the app.
 */
export type RoleAccessOverrides = Partial<Record<Role, Partial<Record<Area, Access>>>>;

let overrides: RoleAccessOverrides = {};

/** Set once after settings load (see useSettingsStore) and again whenever the matrix is saved. */
export function setRoleAccessOverrides(next: RoleAccessOverrides | undefined): void {
  overrides = next ?? {};
}

export function effectiveAccess(role: Role, area: Area): Access {
  return overrides[role]?.[area] ?? ROLE_ACCESS[role][area];
}

export function roleCan(role: Role | undefined, area: Area, access: Exclude<Access, 'none'> = 'read'): boolean {
  if (!role) return false;
  return RANK[effectiveAccess(role, area)] >= RANK[access];
}

/**
 * v2 (docs/v2/01-personas.md storekeeper row: "R (W on units/barcodes)"): a storekeeper's
 * `inventory`/`catalog` access is write on stock, but the product form's pricing/tax/accounts tabs
 * stay read-only for them — only the units & barcodes tab is editable. Admin/manager always can.
 */
export function roleCanEditUnitsOnly(role: Role | undefined): boolean {
  return role === 'storekeeper';
}

/**
 * v2 (docs/v2/01-personas.md storekeeper: "Receiving is the same action as posting the supplier
 * bill... can't be done without seeing or entering prices" is the *old* v1 problem; v2 gives a
 * price-hidden receiving mode for storekeepers). Purchase prices/costs are hidden from anyone
 * without pricing visibility — admins, managers and accountants can always see them.
 */
export function roleCanSeePurchasePrices(role: Role | undefined): boolean {
  return role !== 'storekeeper' && role !== 'cashier';
}

/**
 * `settings.restoreBackup` (docs/v2/14-platform.md §4 "الاستعادة"). The app only has area+access
 * permissions today (no fine-grained permission strings), so this is modeled as "write access to
 * settings" — i.e. admin/manager, the same roles that can already change store settings. If a real
 * per-permission matrix is added later, this is the one place to swap the check.
 */
export function roleCanRestoreBackup(role: Role | undefined): boolean {
  return roleCan(role, 'settings', 'write');
}

/**
 * `sales.overrideCreditLimit` (docs/v2/02-accounting-review.md D3). Same modeling approach as
 * `roleCanRestoreBackup`: no fine-grained permission matrix yet, so this is "write access to
 * accounting" — i.e. admin/manager/accountant, who can already see and adjust a customer's
 * account. A cashier (POS-only) can never override it.
 */
export function roleCanOverrideCreditLimit(role: Role | undefined): boolean {
  return roleCan(role, 'accounting', 'write') || roleCan(role, 'parties', 'write');
}
