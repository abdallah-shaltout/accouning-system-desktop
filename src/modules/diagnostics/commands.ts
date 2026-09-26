import type { AppRoute } from '@/modules/core/types/route';
import type { PaletteSearchProvider } from '@/modules/core/types/commandPalette';
import { normalizeArabic } from '@/modules/core/helpers/search';

/**
 * Command palette registration for diagnostics (18.B5). `/dev/diagnostics` only exists in dev
 * builds (see `modules/core/routes/index.ts`), so this provider is a no-op in production —
 * matching the `/dev/ui` gallery's own "not linked, dev-only" treatment.
 */

const DEV_PAGES: { title: string; to: AppRoute }[] = [{ title: 'التشخيص', to: { name: 'dev-diagnostics' } }];

export const searchProviders: PaletteSearchProvider[] = [
  {
    id: 'diagnostics-dev',
    group: 'pages',
    async search(query, _signal) {
      if (!import.meta.env.DEV) return [];
      const q = normalizeArabic(query);
      return DEV_PAGES.filter((p) => normalizeArabic(p.title).includes(q)).map((p) => ({
        id: `diagnostics:${p.to.name}`,
        group: 'pages' as const,
        title: p.title,
        to: p.to,
      }));
    },
  },
];
