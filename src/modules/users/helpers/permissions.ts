import type { Access, Area, Role } from '../types';

/**
 * Role presets (domain_model.md §10) — replaces the reference system's 100+ permission strings.
 *   admin      everything
 *   manager    everything except user management
 *   accountant accounting, reports, payments (write); sales/purchases/inventory/parties read-only
 *   cashier    POS/sales only, read-only inventory & customers, no accounting/reports
 */
export const ROLE_ACCESS: Record<Role, Record<Area, Access>> = {
  admin: {
    dashboard: 'write', pos: 'write', sales: 'write', inventory: 'write', parties: 'write',
    purchases: 'write', accounting: 'write', payments: 'write', reports: 'write', users: 'write', settings: 'write',
  },
  manager: {
    dashboard: 'write', pos: 'write', sales: 'write', inventory: 'write', parties: 'write',
    purchases: 'write', accounting: 'write', payments: 'write', reports: 'write', users: 'none', settings: 'write',
  },
  accountant: {
    dashboard: 'read', pos: 'none', sales: 'read', inventory: 'read', parties: 'read',
    purchases: 'read', accounting: 'write', payments: 'write', reports: 'write', users: 'none', settings: 'none',
  },
  cashier: {
    dashboard: 'read', pos: 'write', sales: 'write', inventory: 'read', parties: 'read',
    purchases: 'none', accounting: 'none', payments: 'none', reports: 'none', users: 'none', settings: 'none',
  },
};

const RANK: Record<Access, number> = { none: 0, read: 1, write: 2 };

export function roleCan(role: Role | undefined, area: Area, access: Exclude<Access, 'none'> = 'read'): boolean {
  if (!role) return false;
  return RANK[ROLE_ACCESS[role][area]] >= RANK[access];
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
