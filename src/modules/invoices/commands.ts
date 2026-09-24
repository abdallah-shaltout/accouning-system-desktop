import type { RouteLocationNormalizedLoaded, Router } from 'vue-router';
import { formatMoney } from '@/modules/core/helpers/format';
import type { PaletteCommand, PaletteSearchProvider } from '@/modules/core/types/commandPalette';
import { getInvoices } from './services/invoiceService';

/**
 * Command palette registration for invoices (docs/v2/14-platform.md §2 "الفواتير" group + `#`
 * document-number prefix + context commands on the invoice detail page).
 */

export const searchProviders: PaletteSearchProvider[] = [
  {
    id: 'invoices',
    group: 'invoices',
    permission: { area: 'sales', access: 'read' },
    async search(query, signal) {
      const list = await getInvoices({ search: query });
      if (signal.aborted) return [];
      return list.slice(0, 8).map((inv) => ({
        id: `invoice:${inv.id}`,
        group: 'invoices' as const,
        title: inv.number,
        subtitle: `${inv.customerName ?? 'عميل نقدي'} — ${formatMoney(inv.grandTotal)}`,
        keywords: inv.customerName,
        to: `/invoices/${inv.id}`,
      }));
    },
  },
];

/**
 * `when(route)` context commands (docs/v2/14-platform.md §2 "Context: commands can declare
 * `when(route)`. On an invoice page, 'طباعة هذه الفاتورة' and 'إنشاء إشعار دائن' appear first").
 * Built as a function of `router` (same pattern as `buildPageCommands`) so `run`/`to` can read
 * `router.currentRoute.value` at click time — `PaletteCommand.to`/`run` are static, only `when` is
 * reactive to the route, so the id has to be resolved lazily like this rather than baked in upfront.
 */
export function buildContextCommands(router: Router): PaletteCommand[] {
  const isInvoicePage = (route: RouteLocationNormalizedLoaded) => route.name === 'invoice';
  return [
    {
      id: 'context:invoice-print',
      group: 'actions',
      title: 'طباعة هذه الفاتورة',
      when: (route) => isInvoicePage(route),
      run: () => router.push(`/print/invoices/${router.currentRoute.value.params.id}`),
      permission: { area: 'sales', access: 'read' },
    },
    {
      id: 'context:invoice-credit-note',
      group: 'actions',
      title: 'إنشاء إشعار دائن',
      when: (route) => isInvoicePage(route),
      run: () => router.push(`/invoices/${router.currentRoute.value.params.id}/refund`),
      permission: { area: 'sales', access: 'write' },
    },
  ];
}

export const commands: PaletteCommand[] = [
  {
    id: 'action:invoice-new',
    group: 'actions',
    title: 'فاتورة مبيعات جديدة',
    keywords: 'فاتورة بيع invoice',
    to: '/sales/invoices/new',
    permission: { area: 'sales', access: 'write' },
  },
];
