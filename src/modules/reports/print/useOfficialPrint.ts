import { ref } from 'vue';
import { formatDateTime } from '@/modules/core/helpers/format';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { DEFAULT_SIGNATURES } from './build';
import type { PrintBlock, ReportDocument, ReportPrintSpec } from './types';

export interface OfficialPrintInput extends ReportPrintSpec {
  title: string;
  subtitle?: string;
  /** Leading meta cells (period / as-of / document number…), before the spec's own `meta`. */
  leadMeta?: { label: string; value: string }[];
  /** Currency code shown in the meta strip (defaults to the company currency). */
  currency?: string;
  /** Signature box titles when the default accountant / finance manager / general manager set doesn't fit. */
  signatureTitles?: string[];
  badge?: string;
  /** Label for the signed-in user's meta cell — "أُعدّ بواسطة" for reports, "طُبع بواسطة" on documents someone else created. */
  userLabel?: string;
}

function widestTable(blocks: PrintBlock[]): number {
  return blocks.reduce((max, b) => {
    if (b.type === 'table') return Math.max(max, b.columns.length);
    if (b.type === 'columns') return Math.max(max, ...b.columns.map(widestTable));
    return max;
  }, 0);
}

/**
 * Builds the official `ReportDocument` (company letterhead from settings, issue timestamp,
 * prepared-by user, currency) around a page's print spec, and holds the preview dialog's state —
 * shared by `ReportShell` and every other screen that prints a real document instead of itself
 * (`ReportPrintDialog` renders and prints it).
 */
export function useOfficialPrint() {
  const settings = useSettingsStore();
  const auth = useAuthStore();
  const open = ref(false);
  const doc = ref<ReportDocument | null>(null);

  function build(input: OfficialPrintInput): ReportDocument {
    const s = settings.settings;
    const now = new Date().toISOString();
    const currency = input.currency || s?.currency || '';
    return {
      title: input.title,
      subtitle: input.subtitle,
      badge: input.badge ?? 'تقرير رسمي',
      company: { name: s?.storeName ?? '', address: s?.address, phone: s?.phone, vatNumber: s?.vatNumber, commercialRegister: s?.commercialRegister, logo: s?.logo },
      meta: [
        ...(input.leadMeta ?? []),
        ...(input.meta ?? []),
        ...(currency ? [{ label: 'العملة', value: currency }] : []),
        { label: 'تاريخ الإصدار', value: formatDateTime(now) },
        ...(auth.user ? [{ label: input.userLabel ?? 'أُعدّ بواسطة', value: auth.user.name }] : []),
      ],
      blocks: input.blocks,
      signatures: input.signatures ? (input.signatureTitles ?? DEFAULT_SIGNATURES) : [],
      orientation: input.orientation ?? (widestTable(input.blocks) > 7 ? 'landscape' : 'portrait'),
      issuedAt: formatDateTime(now),
      issuedIso: now,
      footerNote: 'تقرير صادر آلياً من النظام — للاستخدام الرسمي',
    };
  }

  function show(input: OfficialPrintInput) {
    doc.value = build(input);
    open.value = true;
  }

  return { open, doc, build, show };
}
