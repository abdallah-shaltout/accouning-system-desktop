<script setup lang="ts">
import { computed } from 'vue';
import { FileDown, FileSpreadsheet, FileText, Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDate, formatDateTime } from '@/modules/core/helpers/format';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { saveTextFile, toCsv, toMarkdown, type ExportTable } from '../helpers/export';

/**
 * Frame shared by every report: title, filter bar, export actions, and a print-only header.
 * Exports operate on `table` — the snapshot of what is currently rendered.
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
</script>

<template>
  <div>
    <PageHeader :title="title" :subtitle="subtitle" back="/reports">
      <template #actions>
        <AppButton size="sm" :icon="Printer" :disabled="loading || !!error" title="طباعة أو حفظ كـ PDF من نافذة الطباعة" @click="print">
          PDF / طباعة
        </AppButton>
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
        <p class="text-[15px] font-semibold">{{ settings.settings?.storeName }}</p>
        <p class="text-[13px]">{{ title }}</p>
        <p class="text-[11px] text-print-muted">{{ period }} · <FileText class="inline size-3" /> {{ formatDateTime(new Date().toISOString()) }}</p>
      </div>
      <p class="no-print mb-3 text-xs text-text-secondary">{{ period }}</p>
      <slot />
    </div>
  </div>
</template>
