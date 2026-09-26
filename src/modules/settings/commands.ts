import type { AppRoute } from '@/modules/core/types/route';
import type { PaletteResult, PaletteSearchProvider } from '@/modules/core/types/commandPalette';
import { normalizeArabic } from '@/modules/core/helpers/search';

/**
 * Command palette registration for settings (docs/v2/14-platform.md §2 "الإعدادات" group). These
 * pages are already reachable through the generic "الصفحات" (pages) provider (`buildPageCommands`
 * scans every named route), but the doc calls out a distinct settings group, so this mirrors the
 * settings routes under their own group label instead of leaving them only under "pages".
 */

const SETTINGS_PAGES: { title: string; to: AppRoute }[] = [
  { title: 'عام', to: { name: 'settings-general' } },
  { title: 'الضرائب', to: { name: 'settings-taxes' } },
  { title: 'طرق الدفع', to: { name: 'settings-payment-methods' } },
  { title: 'الطباعة', to: { name: 'settings-printing' } },
  { title: 'المنتجات', to: { name: 'settings-products' } },
  { title: 'الفروع', to: { name: 'settings-branches' } },
  { title: 'مراكز التكلفة', to: { name: 'settings-cost-centers' } },
  { title: 'العملات', to: { name: 'settings-currencies' } },
  { title: 'التوصيات', to: { name: 'settings-recommendations' } },
  { title: 'المستخدمون والأدوار', to: { name: 'settings-roles' } },
  { title: 'سجل التدقيق', to: { name: 'settings-audit-log' } },
  { title: 'المظهر', to: { name: 'settings-appearance' } },
  { title: 'اختصارات لوحة المفاتيح', to: { name: 'settings-keyboard-shortcuts' } },
  { title: 'النسخ الاحتياطي', to: { name: 'settings-backup' } },
  { title: 'قوالب الطباعة', to: { name: 'settings-templates' } },
  { title: 'حول / الدعم', to: { name: 'settings-about' } },
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
        .map((s) => ({ id: `settings:${s.to.name}`, group: 'settings' as const, title: s.title, to: s.to }));
      return results;
    },
  },
];
