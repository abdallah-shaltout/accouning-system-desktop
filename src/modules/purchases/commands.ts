import type { RouteLocationNormalizedLoaded, Router } from 'vue-router';
import { formatMoney } from '@/modules/core/helpers/format';
import type { PaletteCommand, PaletteSearchProvider } from '@/modules/core/types/commandPalette';
import { getPurchaseOrders } from './services/purchaseService';

/** Command palette registration for purchases (docs/v2/14-platform.md §2 "المشتريات" group). */

export const searchProviders: PaletteSearchProvider[] = [
  {
    id: 'purchases',
    group: 'purchases',
    permission: { area: 'purchases', access: 'read' },
    async search(query, signal) {
      const list = await getPurchaseOrders({ search: query });
      if (signal.aborted) return [];
      return list.slice(0, 8).map((po) => ({
        id: `purchase:${po.id}`,
        group: 'purchases' as const,
        title: po.number,
        subtitle: `${po.supplierName} — ${formatMoney(po.grandTotal)}`,
        keywords: po.supplierName,
        to: { name: 'purchase', params: { id: po.id } },
      }));
    },
  },
];

export function buildContextCommands(router: Router): PaletteCommand[] {
  const isPurchasePage = (route: RouteLocationNormalizedLoaded) => route.name === 'purchase';
  return [
    {
      id: 'context:purchase-print',
      group: 'actions',
      title: 'طباعة أمر الشراء',
      when: isPurchasePage,
      run: () => {
        const r = router.currentRoute.value;
        if (r.name !== 'purchase') return;
        router.push({ name: 'purchase-print', params: { id: r.params.id } });
      },
      permission: { area: 'purchases', access: 'read' },
    },
    {
      id: 'context:purchase-receive',
      group: 'actions',
      title: 'استلام أمر الشراء',
      when: isPurchasePage,
      run: () => {
        const r = router.currentRoute.value;
        if (r.name !== 'purchase') return;
        router.push({ name: 'purchase-receive', params: { id: r.params.id } });
      },
      permission: { area: 'purchases', access: 'write' },
    },
    {
      id: 'context:purchase-return',
      group: 'actions',
      title: 'مرتجع مشتريات',
      when: isPurchasePage,
      run: () => {
        const r = router.currentRoute.value;
        if (r.name !== 'purchase') return;
        router.push({ name: 'purchase-return', params: { id: r.params.id } });
      },
      permission: { area: 'purchases', access: 'write' },
    },
  ];
}

export const commands: PaletteCommand[] = [
  {
    id: 'action:purchase-new',
    group: 'actions',
    title: 'أمر شراء جديد',
    keywords: 'مشتريات purchase',
    to: { name: 'purchase-new' },
    permission: { area: 'purchases', access: 'write' },
  },
];
