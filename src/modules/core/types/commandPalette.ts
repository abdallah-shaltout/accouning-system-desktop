import type { Component } from 'vue';
import type { RouteLocationNormalizedLoaded, RouteLocationRaw } from 'vue-router';
import type { Access, Area } from '@/modules/users/types';

/**
 * Command palette contract (docs/v2/14-platform.md §2). Each module adds its own `commands.ts`
 * exporting `commands: PaletteCommand[]` and `searchProviders: PaletteSearchProvider[]` — this
 * Phase 0 track builds the shell/registry/UI + a couple of example providers to prove the shape;
 * later phases add the rest (see `docs/v2/15-action-plan.md` Phase 13b).
 */

export type PaletteGroup =
  | 'actions'
  | 'pages'
  | 'customers'
  | 'suppliers'
  | 'products'
  | 'invoices'
  | 'purchases'
  | 'vouchers'
  | 'journal'
  | 'accounts'
  | 'reports'
  | 'settings'
  | 'help';

export const GROUP_LABEL: Record<PaletteGroup, string> = {
  actions: 'الأوامر',
  pages: 'الصفحات',
  customers: 'العملاء',
  suppliers: 'الموردون',
  products: 'المنتجات',
  invoices: 'الفواتير',
  purchases: 'المشتريات',
  vouchers: 'السندات',
  journal: 'القيود',
  accounts: 'الحسابات',
  reports: 'التقارير',
  settings: 'الإعدادات',
  help: 'مساعدة',
};

/** A permission gate: the current user must have at least `access` on `area`. Omit for no gating. */
export interface PalettePermission {
  area: Area;
  access?: Exclude<Access, 'none'>;
}

export interface PaletteResult {
  id: string;
  group: PaletteGroup;
  title: string;
  subtitle?: string;
  icon?: Component;
  keywords?: string;
  /** Navigates here when chosen. Mutually exclusive with `run` in practice, but both are allowed. */
  to?: RouteLocationRaw;
  /** Runs an arbitrary action (theme toggle, open a modal, etc.) instead of/in addition to navigating. */
  run?: () => void;
  permission?: PalettePermission;
}

export interface PaletteCommand extends PaletteResult {
  /** Only show this command when `when` returns true for the current route (context-aware actions). */
  when?: (route: RouteLocationNormalizedLoaded) => boolean;
}

export interface PaletteSearchProvider {
  id: string;
  group: PaletteGroup;
  permission?: PalettePermission;
  /** Called with the normalized query (prefix already stripped) and an AbortSignal for cancellation. */
  search: (query: string, signal: AbortSignal) => Promise<PaletteResult[]>;
}

/** Prefixes narrow the search to one kind of result (docs/v2/14-platform.md §2). */
export type PalettePrefix = '>' | '@' | '#' | '$' | '?';

export const PREFIX_GROUPS: Record<PalettePrefix, PaletteGroup[]> = {
  '>': ['actions'],
  '@': ['customers', 'suppliers'],
  '#': ['invoices', 'purchases', 'journal', 'vouchers'],
  '$': ['accounts'],
  '?': ['help'],
};
