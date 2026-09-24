import type { RouteLocationNormalizedLoaded, Router } from 'vue-router';
import { formatMoney } from '@/modules/core/helpers/format';
import type { PaletteCommand, PaletteSearchProvider } from '@/modules/core/types/commandPalette';
import { getAccounts, getJournalEntries } from './services/accountingService';

/**
 * Command palette registration for accounting (docs/v2/14-platform.md §2 "القيود" and "الحسابات"
 * groups + `$` account prefix + `#` journal-number prefix).
 */

export const searchProviders: PaletteSearchProvider[] = [
  {
    id: 'journal',
    group: 'journal',
    permission: { area: 'accounting', access: 'read' },
    async search(query, signal) {
      const list = await getJournalEntries({ search: query });
      if (signal.aborted) return [];
      return list.slice(0, 8).map((e) => ({
        id: `journal:${e.id}`,
        group: 'journal' as const,
        title: e.number,
        subtitle: e.description,
        to: `/accounting/journal/${e.id}`,
      }));
    },
  },
  {
    id: 'accounts',
    group: 'accounts',
    permission: { area: 'accounting', access: 'read' },
    async search(query) {
      const q = query.trim().toLowerCase();
      const list = await getAccounts();
      return list
        .filter((a) => !a.isGroup && (a.code.toLowerCase().includes(q) || a.name.toLowerCase().includes(q)))
        .slice(0, 8)
        .map((a) => ({
          id: `account:${a.id}`,
          group: 'accounts' as const,
          title: a.name,
          subtitle: `${a.code} — الرصيد ${formatMoney(a.balance)}`,
          keywords: a.code,
          to: `/reports/ledger?account=${a.id}`,
        }));
    },
  },
];

export function buildContextCommands(_router: Router): PaletteCommand[] {
  return [
    {
      id: 'context:journal-print',
      group: 'actions',
      title: 'طباعة هذا القيد',
      when: (route: RouteLocationNormalizedLoaded) => route.name === 'journal-entry',
      run: () => window.print(),
      permission: { area: 'accounting', access: 'read' },
    },
  ];
}

export const commands: PaletteCommand[] = [
  {
    id: 'action:journal-entry-new',
    group: 'actions',
    title: 'قيد يدوي جديد',
    keywords: 'محاسبة journal قيد',
    to: '/accounting/journal/new',
    permission: { area: 'accounting', access: 'write' },
  },
  {
    id: 'action:chart-of-accounts',
    group: 'actions',
    title: 'دليل الحسابات',
    to: '/accounting/accounts',
    permission: { area: 'accounting', access: 'read' },
  },
];
