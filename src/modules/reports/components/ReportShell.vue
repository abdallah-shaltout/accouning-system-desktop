<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { FileDown, FileSpreadsheet, Lightbulb, Printer, Table2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import InsightChip from '@/modules/core/components/insights/InsightChip.vue';
import { useInsights } from '@/modules/core/controllers/useInsights';
import { useToast } from '@/modules/core/controllers/useToast';
import { exportXlsx } from '@/modules/core/helpers/exportXlsx';
import { formatDate, formatDateTime } from '@/modules/core/helpers/format';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { getBranches, getCostCenters } from '@/modules/settings/services/branchesService';
import { saveTextFile, toCsv, toMarkdown, type ExportTable } from '../helpers/export';
import { blocksFromTable } from '../print/build';
import type { ReportPrintSpec } from '../print/types';
import { useOfficialPrint } from '../print/useOfficialPrint';
import ReportPrintDialog from './ReportPrintDialog.vue';

/**
 * v2 (docs/v2/13-reports.md §1) — the shared report frame: title, filter bar (period, comparison,
 * branch/cost-center/currency — added via the `#filters` slot by each page using
 * `useReportFilters`/`useReportRange`), an insights box and export actions.
 * Excel/CSV/Markdown exports operate on `table` — the snapshot of what is currently rendered.
 *
 * Print / PDF is a real document render, never the on-screen component: the page's `print` spec (or,
 * when a page has none, its `table` converted by `blocksFromTable`) becomes a `ReportDocument` with
 * the company letterhead, period/issue/currency/prepared-by strip and optional signatures, previewed
 * in `ReportPrintDialog` and printed from there (HTML, `print/renderHtml.ts`) or saved as a native
 * PDF (Typst, `src-tauri/templates/report.typ`). Ctrl+P opens the same preview.
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
   * v2 (docs/v2/13 §1 "Insights box"): a headline + 2–4 metrics/recommendations. Every report page
   * still passes its own small per-page summary here (kept — it's specific to what the report just
   * computed, e.g. "الميزان متوازن"), but when `ruleKeys` is also given, the real rule-catalogue
   * insights relevant to this report's domain (docs/v2/11 Part D) are fetched and shown alongside it.
   */
  insights?: { headline: string; metrics?: { label: string; value: string }[] } | null;
  /** Insight-engine rule keys relevant to this report (e.g. `['vat-deadline']` on the VAT report) — see the pages for the mapping. Omit to show only the plain per-page summary above. */
  ruleKeys?: string[];
  /** The printed document's body (blocks + extra meta). Omit to print `table` in the official layout. */
  print?: ReportPrintSpec | null;
}>();
defineEmits<{ retry: [] }>();

const toast = useToast();
const settings = useSettingsStore();
const route = useRoute();
const router = useRouter();

// v2 (docs/v2/13 §1 "insights box"): real insight-engine hits for this report's domain, in addition
// to the page's own ad-hoc summary. Role-filtered by the signed-in user like every other insight
// surface; capped at 3 so it stays a glance, not another table.
const { insights: engineInsights } = useInsights({ limit: 50 });
const relevantInsights = computed(() => {
  if (!props.ruleKeys?.length) return [];
  const keys = new Set(props.ruleKeys);
  return engineInsights.value.filter((i) => keys.has(i.ruleKey)).slice(0, 3);
});

const period = computed(() => {
  if (props.asOf) return `كما في ${formatDate(props.asOf)}`;
  if (props.from || props.to) return `من ${props.from ? formatDate(props.from) : 'البداية'} إلى ${props.to ? formatDate(props.to) : 'اليوم'}`;
  return 'كل الفترات';
});

const fileBase = computed(() => `${props.title} ${props.asOf ?? [props.from, props.to].filter(Boolean).join('_')}`.trim());

function withMeta(t: ExportTable): ExportTable {
  return { ...t, meta: [settings.settings?.storeName ?? '', period.value, `أُنشئ في ${formatDateTime(new Date().toISOString())}`, ...(t.meta ?? [])] };
}

// ---- Official print / PDF ----------------------------------------------------------------------

const { open: printOpen, doc: printDoc, show: showPrint } = useOfficialPrint();
const canPrint = computed(() => !props.loading && !props.error && !!(props.print ?? props.table));

/** Period cells for the meta strip, from the same props the on-screen period line uses. */
function periodMeta(): { label: string; value: string }[] {
  if (props.asOf) return [{ label: 'كما في', value: formatDate(props.asOf) }];
  if (props.from || props.to)
    return [
      { label: 'الفترة من', value: props.from ? formatDate(props.from) : 'البداية' },
      { label: 'الفترة إلى', value: props.to ? formatDate(props.to) : formatDate(new Date().toISOString()) },
    ];
  return [{ label: 'الفترة', value: 'كل الفترات' }];
}

/** Active dimension filters (docs/v2/10 §4) — printed so a filtered report can't pass for the whole company. */
async function dimensionMeta(): Promise<{ label: string; value: string }[]> {
  const out: { label: string; value: string }[] = [];
  const branchId = typeof route.query.branchId === 'string' ? route.query.branchId : '';
  const costCenterId = typeof route.query.costCenterId === 'string' ? route.query.costCenterId : '';
  if (branchId) out.push({ label: 'الفرع', value: (await getBranches()).find((b) => b.id === branchId)?.name ?? branchId });
  if (costCenterId) out.push({ label: 'مركز التكلفة', value: (await getCostCenters()).find((c) => c.id === costCenterId)?.name ?? costCenterId });
  return out;
}

async function openPrint() {
  if (!canPrint.value) return;
  const spec: ReportPrintSpec = props.print ?? { blocks: blocksFromTable(props.table!, props.insights?.metrics) };
  try {
    showPrint({
      ...spec,
      title: spec.title ?? props.title,
      subtitle: spec.subtitle ?? props.subtitle,
      leadMeta: periodMeta(),
      meta: [...(spec.meta ?? []), ...(await dimensionMeta())],
      currency: typeof route.query.currency === 'string' ? route.query.currency : undefined,
    });
  } catch (err) {
    toast.error(err, 'تعذر تجهيز التقرير للطباعة');
  }
}

// `?print=1` (e.g. the journal list's "دفتر اليومية PDF" button) opens the preview as soon as the
// report has loaded, then drops the flag so a refresh/back doesn't pop it again.
watch(
  canPrint,
  (ok) => {
    if (!ok || route.query.print !== '1') return;
    void router.replace({ query: { ...route.query, print: undefined } });
    void openPrint();
  },
  { immediate: true },
);

// Ctrl+P prints the official document, not the screen. Matched on `code` so it also works with an
// Arabic keyboard layout (where `key` is "ح").
function onKeydown(e: KeyboardEvent) {
  if (!(e.ctrlKey || e.metaKey) || e.code !== 'KeyP' || printOpen.value) return;
  e.preventDefault();
  void openPrint();
}
onMounted(() => window.addEventListener('keydown', onKeydown));
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));

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
        <AppButton size="sm" :icon="Printer" :disabled="!canPrint" title="معاينة النسخة الرسمية ثم الطباعة أو الحفظ PDF (Ctrl+P)" data-testid="report-print-open" @click="openPrint">
          طباعة / PDF
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
    <div v-else>
      <p class="mb-3 text-xs text-text-secondary">{{ period }}</p>

      <!-- Insights box (docs/v2/13 §1): the page's own per-page summary, plus (when `ruleKeys` is
           given) the real rule-catalogue insights for this report's domain (docs/v2/11 Part D). -->
      <div v-if="insights" class="no-print mb-3 flex items-start gap-3 rounded-xl border border-primary/25 bg-primary/5 p-3.5">
        <Lightbulb class="mt-0.5 size-4.5 shrink-0 text-primary" :stroke-width="1.75" />
        <div class="min-w-0">
          <p class="text-body font-medium">{{ insights.headline }}</p>
          <div v-if="insights.metrics?.length" class="mt-1.5 flex flex-wrap gap-x-4 gap-y-1 text-tiny text-text-secondary">
            <span v-for="m in insights.metrics" :key="m.label"><span class="text-text-secondary">{{ m.label }}:</span> <span class="num font-medium text-text-primary">{{ m.value }}</span></span>
          </div>
        </div>
      </div>
      <div v-if="relevantInsights.length" class="no-print mb-4 flex flex-wrap gap-1.5">
        <InsightChip v-for="i in relevantInsights" :key="i.id" :insight="i" />
      </div>

      <slot />
    </div>

    <ReportPrintDialog v-model:open="printOpen" :doc="printDoc" :file-name="fileBase" />
  </div>
</template>
