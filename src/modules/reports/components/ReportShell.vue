<script setup lang="ts">
import { computed } from 'vue';
import { FileDown, FileSpreadsheet, FileText, Lightbulb, Printer, Table2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { exportXlsx } from '@/modules/core/helpers/exportXlsx';
import { formatDate, formatDateTime } from '@/modules/core/helpers/format';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { saveTextFile, toCsv, toMarkdown, type ExportTable } from '../helpers/export';

/**
 * v2 (docs/v2/13-reports.md §1) — the shared report frame: title, filter bar (period, comparison,
 * branch/cost-center/currency — added via the `#filters` slot by each page using
 * `useReportFilters`/`useReportRange`), an insights box, export actions and a print-only header.
 * Exports operate on `table` — the snapshot of what is currently rendered.
 *
 * PDF export: Phase 11b's generic report Typst template hasn't landed yet (only the invoice
 * template exists — `pdfService`'s `PdfDocumentKind` is `'invoice'` only), so this still falls back
 * to the browser print route exactly like v1. TODO(phase 11b): once a generic report template
 * lands, swap the "PDF / طباعة" button to call `pdfService.renderAndSave('report', …)` in Tauri and
 * keep the print-window fallback only for the browser dev server.
 */
const props = defineProps<{
  title: string;
  subtitle?: string;
  from?: string;
  to?: string;
  /** Single "as of" date (balance sheet, inventory). */
  asOf?: string;
  loading?: boolean;
  error?: string | null;
  table?: ExportTable;
  /**
   * v2 (docs/v2/13 §1 "Insights box"): a headline + 2–4 metrics/recommendations, in the shape
   * Phase 10's `ReportInsights`/insight engine will eventually produce. Phase 10 (Home & analytics)
   * hasn't merged yet at the time this shell was built, so pages pass a small ad-hoc summary here
   * (or nothing) instead of wiring the real engine.
   */
  insights?: { headline: string; metrics?: { label: string; value: string }[] } | null;
}>();
defineEmits<{ retry: [] }>();

const toast = useToast();
const settings = useSettingsStore();

const period = computed(() => {
  if (props.asOf) return `كما في ${formatDate(props.asOf)}`;
  if (props.from || props.to) return `من ${props.from ? formatDate(props.from) : 'البداية'} إلى ${props.to ? formatDate(props.to) : 'اليوم'}`;
  return 'كل الفترات';
});

const fileBase = computed(() => `${props.title} ${props.asOf ?? [props.from, props.to].filter(Boolean).join('_')}`.trim());

function withMeta(t: ExportTable): ExportTable {
  return { ...t, meta: [settings.settings?.storeName ?? '', period.value, `أُنشئ في ${formatDateTime(new Date().toISOString())}`, ...(t.meta ?? [])] };
}

function print() {
  window.print();
}

async function exportAs(kind: 'csv' | 'md') {
  if (!props.table) return;
  try {
    const t = withMeta(props.table);
    const ok = await saveTextFile(`${fileBase.value}.${kind}`, kind === 'csv' ? toCsv(t) : toMarkdown(t), kind);
    if (ok) toast.success(kind === 'csv' ? 'تم تصدير CSV' : 'تم تصدير Markdown', `${fileBase.value}.${kind}`);
  } catch (err) {
    toast.error(err, 'تعذر التصدير');
  }
}

/** Excel export (docs/v2/13 §1 "real … Excel (formatted, with SUM formulas)"): built from the same `table` snapshot every other export uses, via Phase 0's `exportXlsx`. */
async function exportExcel() {
  if (!props.table) return;
  try {
    const t = props.table;
    await exportXlsx({
      fileName: fileBase.value,
      sheetName: t.title.slice(0, 31) || 'تقرير',
      columns: t.columns.map((label, i) => ({ key: String(i), label, numeric: typeof t.rows[0]?.[i] === 'number' })),
      rows: t.rows.map((row) => Object.fromEntries(row.map((v, i) => [String(i), v]))),
    });
    toast.success('تم تصدير Excel', `${fileBase.value}.xlsx`);
  } catch (err) {
    toast.error(err, 'تعذر التصدير');
  }
}
</script>

<template>
  <div>
    <PageHeader :title="title" :subtitle="subtitle" back="/reports">
      <template #actions>
        <AppButton size="sm" :icon="Printer" :disabled="loading || !!error" title="طباعة أو حفظ كـ PDF من نافذة الطباعة — TODO(phase 11b): قالب تقرير Typst عام" @click="print">
          PDF / طباعة
        </AppButton>
        <AppButton size="sm" :icon="Table2" :disabled="!table || loading" @click="exportExcel">Excel</AppButton>
        <AppButton size="sm" :icon="FileSpreadsheet" :disabled="!table || loading" @click="exportAs('csv')">CSV</AppButton>
        <AppButton size="sm" :icon="FileDown" :disabled="!table || loading" @click="exportAs('md')">Markdown</AppButton>
      </template>
    </PageHeader>

    <div v-if="$slots.filters" class="no-print mb-4 flex flex-wrap items-end gap-3 rounded-xl border border-border bg-surface p-3">
      <slot name="filters" />
    </div>

    <ErrorState v-if="error" :message="error" @retry="$emit('retry')" />
    <div v-else-if="loading" class="space-y-3"><SkeletonBlock :lines="10" height="h-9" /></div>
    <div v-else class="print-root">
      <!-- Print-only letterhead -->
      <div class="mb-4 hidden border-b-2 border-black pb-3 print:block">
        <p class="text-lead font-semibold">{{ settings.settings?.storeName }}</p>
        <p class="text-body">{{ title }}</p>
        <p class="text-tiny text-print-muted">{{ period }} · <FileText class="inline size-3" /> {{ formatDateTime(new Date().toISOString()) }}</p>
      </div>
      <p class="no-print mb-3 text-xs text-text-secondary">{{ period }}</p>

      <!-- Insights box (docs/v2/13 §1) — TODO(phase 10): wire into the real insight engine (rule
           catalogue + dismiss/snooze) once it merges; this is a plain per-page summary until then. -->
      <div v-if="insights" class="no-print mb-4 flex items-start gap-3 rounded-xl border border-primary/25 bg-primary/5 p-3.5">
        <Lightbulb class="mt-0.5 size-4.5 shrink-0 text-primary" :stroke-width="1.75" />
        <div class="min-w-0">
          <p class="text-body font-medium">{{ insights.headline }}</p>
          <div v-if="insights.metrics?.length" class="mt-1.5 flex flex-wrap gap-x-4 gap-y-1 text-tiny text-text-secondary">
            <span v-for="m in insights.metrics" :key="m.label"><span class="text-text-secondary">{{ m.label }}:</span> <span class="num font-medium text-text-primary">{{ m.value }}</span></span>
          </div>
        </div>
      </div>

      <slot />
    </div>
  </div>
</template>
