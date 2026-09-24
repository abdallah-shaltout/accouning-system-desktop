/**
 * Example command-palette providers for this Phase 0 track (docs/v2/14-platform.md §2). Proves
 * the `PaletteSearchProvider`/`PaletteCommand` shape end-to-end with:
 *   - a "pages" provider (every named route with a title, driven by the router itself — no module
 *     needs to hand-list its pages)
 *   - a "customers" data provider (`@` prefix, using `normalizeArabic` through `getCustomers`)
 *   - a "products" data provider (barcode/SKU/name)
 *   - a small set of global actions (new sale, toggle theme…)
 *
 * Later phases replace/extend this with each module's own `commands.ts` (Phase 13b, per the
 * action plan) — this file stays as the "pages" provider plus maybe folds into settings/core,
 * but customers/products get their real providers from Phase 4/6 instead.
 */
import type { Router } from 'vue-router';
import { getCustomers, getSuppliers } from '@/modules/parties/services/partyService';
import { getProducts } from '@/modules/products/services/productService';
import { toggleTheme } from '../controllers/useTheme';
import type { PaletteCommand, PaletteSearchProvider } from '../types/commandPalette';

export function buildPageCommands(router: Router): PaletteCommand[] {
  return router
    .getRoutes()
    .filter(
      (r) =>
        r.meta?.title &&
        r.name &&
        r.meta.layout !== 'blank' &&
        !r.path.includes(':') && // skip detail routes that need a param (…/:id) — list/top-level pages only for this example
        !String(r.name).match(/-(new|edit)$/),
    )
    .map((r) => ({
      id: `page:${String(r.name)}`,
      group: 'pages' as const,
      title: String(r.meta.title),
      subtitle: r.meta.section as string | undefined,
      to: r.path,
      permission: r.meta.area ? { area: r.meta.area as any, access: (r.meta.access as any) ?? 'read' } : undefined,
    }));
}

/**
 * `to` commands are navigated by the palette panel itself; only side-effect-only commands (no
 * `to`) need a `run`. `router` is accepted for symmetry with `buildPageCommands` and future
 * commands that both run a side effect and navigate.
 */
export function buildActionCommands(_router: Router): PaletteCommand[] {
  return [
    {
      id: 'action:new-sale',
      group: 'actions',
      title: 'بيع جديد (نقطة البيع)',
      keywords: 'pos بيع كاشير',
      to: '/pos',
      permission: { area: 'pos', access: 'write' },
    },
    {
      id: 'action:new-customer',
      group: 'actions',
      title: 'عميل جديد',
      to: '/customers',
      permission: { area: 'parties', access: 'write' },
    },
    {
      id: 'action:toggle-theme',
      group: 'actions',
      title: 'تبديل المظهر (فاتح/داكن)',
      run: () => toggleTheme(),
    },
    {
      id: 'action:go-settings',
      group: 'actions',
      title: 'الإعدادات',
      to: '/settings/general',
      permission: { area: 'settings', access: 'read' },
    },
  ];
}

export const customersProvider: PaletteSearchProvider = {
  id: 'customers',
  group: 'customers',
  permission: { area: 'parties', access: 'read' },
  async search(query, signal) {
    const list = await getCustomers({ search: query, includeInactive: false });
    if (signal.aborted) return [];
    return list.slice(0, 8).map((c) => ({
      id: `customer:${c.id}`,
      group: 'customers' as const,
      title: c.name,
      subtitle: c.phone ?? c.vatNumber,
      to: `/customers/${c.id}`,
    }));
  },
};

export const suppliersProvider: PaletteSearchProvider = {
  id: 'suppliers',
  group: 'suppliers',
  permission: { area: 'parties', access: 'read' },
  async search(query, signal) {
    const list = await getSuppliers({ search: query, includeInactive: false });
    if (signal.aborted) return [];
    return list.slice(0, 8).map((s) => ({
      id: `supplier:${s.id}`,
      group: 'suppliers' as const,
      title: s.name,
      subtitle: s.contactPerson ?? s.phone,
      to: `/suppliers/${s.id}`,
    }));
  },
};

export const productsProvider: PaletteSearchProvider = {
  id: 'products',
  group: 'products',
  permission: { area: 'inventory', access: 'read' },
  async search(query, signal) {
    const list = await getProducts({ search: query, includeInactive: false });
    if (signal.aborted) return [];
    return list.slice(0, 8).map((p) => ({
      id: `product:${p.id}`,
      group: 'products' as const,
      title: p.name,
      subtitle: p.barcode ?? p.sku,
      keywords: `${p.sku} ${p.barcode ?? ''}`,
      to: `/products/${p.id}`,
    }));
  },
};
