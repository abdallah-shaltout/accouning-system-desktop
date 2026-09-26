import type { PaletteResult, PaletteSearchProvider } from '@/modules/core/types/commandPalette';
import { normalizeArabic } from '@/modules/core/helpers/search';

/**
 * Command palette registration for settings (docs/v2/14-platform.md §2 "الإعدادات" group). These
 * pages are already reachable through the generic "الصفحات" (pages) provider (`buildPageCommands`
 * scans every named route), but the doc calls out a distinct settings group, so this mirrors the
 * settings routes under their own group label instead of leaving them only under "pages".
 */

const SETTINGS_PAGES: { title: string; to: string }[] = [
  { title: 'عام', to: '/settings/general' },
  { title: 'الضرائب', to: '/settings/taxes' },
  { title: 'طرق الدفع', to: '/settings/payment-methods' },
  { title: 'الطباعة', to: '/settings/printing' },
  { title: 'المنتجات', to: '/settings/products' },
  { title: 'الفروع', to: '/settings/branches' },
  { title: 'مراكز التكلفة', to: '/settings/cost-centers' },
  { title: 'العملات', to: '/settings/currencies' },
  { title: 'التوصيات', to: '/settings/recommendations' },
  { title: 'المستخدمون والأدوار', to: '/settings/roles' },
  { title: 'سجل التدقيق', to: '/settings/audit-log' },
  { title: 'المظهر', to: '/settings/appearance' },
  { title: 'النسخ الاحتياطي', to: '/settings/backup' },
  { title: 'قوالب الطباعة', to: '/settings/templates' },
  { title: 'حول / الدعم', to: '/settings/about' },
];

export const searchProviders: PaletteSearchProvider[] = [
  {
    id: 'settings',
    group: 'settings',
    permission: { area: 'settings', access: 'read' },
    async search(query) {
      const q = normalizeArabic(query);
      const results: PaletteResult[] = SETTINGS_PAGES.filter((s) => normalizeArabic(s.title).includes(q))
        .slice(0, 8)
        .map((s) => ({ id: `settings:${s.to}`, group: 'settings' as const, title: s.title, to: s.to }));
      return results;
    },
  },
];
