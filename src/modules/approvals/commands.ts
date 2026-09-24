import type { PaletteCommand } from '@/modules/core/types/commandPalette';

/** Command palette registration for approvals (docs/v2/14-platform.md §2/§6). */
export const commands: PaletteCommand[] = [
  {
    id: 'action:approvals',
    group: 'actions',
    title: 'طلبات الاعتماد',
    keywords: 'approvals اعتماد موافقة',
    to: '/approvals',
    permission: { area: 'approvals', access: 'write' },
  },
];
