import type { RouteLocationNormalizedLoaded, Router } from 'vue-router';
import { formatMoney } from '@/modules/core/helpers/format';
import type { PaletteCommand, PaletteSearchProvider } from '@/modules/core/types/commandPalette';
import { getVouchers } from './services/voucherService';

/** Command palette registration for vouchers (docs/v2/14-platform.md §2 "السندات" group). */

export const searchProviders: PaletteSearchProvider[] = [
  {
    id: 'vouchers',
    group: 'vouchers',
    permission: { area: 'payments', access: 'read' },
    async search(query, signal) {
      const list = await getVouchers({ search: query });
      if (signal.aborted) return [];
      return list.slice(0, 8).map((v) => ({
        id: `voucher:${v.id}`,
        group: 'vouchers' as const,
        title: v.number,
        subtitle: `${v.description} — ${formatMoney(v.amount)}`,
        to: `/vouchers/${v.id}`,
      }));
    },
  },
];

export function buildContextCommands(router: Router): PaletteCommand[] {
  return [
    {
      id: 'context:voucher-print',
      group: 'actions',
      title: 'طباعة هذا السند',
      when: (route: RouteLocationNormalizedLoaded) => route.name === 'voucher-detail',
      run: () => {
        const r = router.currentRoute.value;
        if (r.name !== 'voucher-detail') return;
        router.push(`/print/vouchers/${r.params.id}`);
      },
      permission: { area: 'payments', access: 'read' },
    },
  ];
}

export const commands: PaletteCommand[] = [
  {
    id: 'action:voucher-new',
    group: 'actions',
    title: 'سند عام جديد',
    keywords: 'سندات voucher قبض صرف',
    to: '/vouchers/new',
    permission: { area: 'payments', access: 'write' },
  },
  {
    id: 'action:payment-received',
    group: 'actions',
    title: 'سند قبض من عميل',
    to: '/payments/new?type=RECEIVED',
    permission: { area: 'payments', access: 'write' },
  },
];
