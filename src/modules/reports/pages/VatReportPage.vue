<script setup lang="ts">
import { computed, watch } from 'vue';
import { CircleAlert, CircleCheck } from '@lucide/vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getVatReport } from '../services/reportService';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getVatReport({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

// v2 (docs/v2/13 §1 "insights box") — TODO(phase 10): wire into the real insight engine once merged.
const insights = computed(() => {
  const d = data.value;
  if (!d) return null;
  return {
    headline: d.netPayable >= 0 ? `مستحق للهيئة ${formatNumber(d.netPayable)} عن هذه الفترة` : `رصيد ضريبي مسترد قدره ${formatNumber(Math.abs(d.netPayable))}`,
    metrics: [
      { label: 'ضريبة المخرجات', value: formatNumber(d.outputVat) },
      { label: 'ضريبة المدخلات', value: formatNumber(d.inputVat) },
    ],
  };
});

const rows = computed(() => {
  const d = data.value;
  if (!d) return [];
  return [
    { label: 'المبيعات الخاضعة للضريبة', sub: 'فواتير المبيعات', ...d.sales, sign: 1 },
    { label: 'مرتجعات المبيعات', sub: 'تُخصم من ضريبة المخرجات', ...d.salesReturns, sign: -1 },
    { label: 'المشتريات الخاضعة للضريبة', sub: 'أوامر الشراء المؤكدة', ...d.purchases, sign: 1 },
    { label: 'مرتجعات المشتريات', sub: 'تُخصم من ضريبة المدخلات', ...d.purchaseReturns, sign: -1 },
  ];
});
const reconciled = computed(() => !!data.value && Math.abs(data.value.outputVat - data.value.ledgerOutput) < 0.01 && Math.abs(data.value.inputVat - data.value.ledgerInput) < 0.01);

/** ZATCA return box labels (docs/v2/02-accounting-review.md D1) — one row per (category, rate). */
const CATEGORY_LABEL: Record<string, string> = { S: 'خاضعة للنسبة الأساسية', Z: 'خاضعة لنسبة الصفر', E: 'معفاة', O: 'خارج نطاق الضريبة' };
const boxes = computed(() => [...(data.value?.salesBoxes ?? [])].sort((a, b) => a.category.localeCompare(b.category) || b.rate - a.rate));

const table = computed<ExportTable | undefined>(() => {
  const d = data.value;
  if (!d) return undefined;
  return {
    title: 'ملخص ضريبة القيمة المضافة',
    columns: ['البند', 'عدد المستندات', 'المبلغ الخاضع', 'الضريبة'],
    rows: [
      ...rows.value.map((r) => [r.label, r.count, r.sign * r.taxable, r.sign * r.vat]),
      ['ضريبة المخرجات', '', '', d.outputVat],
      ['ضريبة المدخلات', '', '', d.inputVat],
      ['صافي الضريبة المستحقة', '', '', d.netPayable],
      ...boxes.value.map((b) => [`${CATEGORY_LABEL[b.category] ?? b.category} (${b.rate}%)`, '', b.net, b.vat]),
    ],
  };
});
</script>

<template>
  <ReportShell
    title="ملخص ضريبة القيمة المضافة"
    subtitle="للاستعانة به عند تقديم الإقرار الضريبي"
    :from="from"
    :to="to"
    :loading="(loading || !ready) && !data"
    :error="error"
    :table="table"
    :insights="insights"
    @retry="reload"
  >
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <template v-if="data">
      <div class="overflow-hidden rounded-xl border border-border">
        <table class="w-full text-body">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2.5 text-start font-medium">البند</th>
              <th class="px-3 py-2.5 text-start font-medium">المستندات</th>
              <th class="px-3 py-2.5 text-start font-medium">المبلغ الخاضع</th>
              <th class="px-4 py-2.5 text-start font-medium">الضريبة</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in rows" :key="r.label" class="border-b border-border">
              <td class="px-4 py-2.5">{{ r.label }}<span class="block text-tiny text-text-secondary">{{ r.sub }}</span></td>
              <td class="px-3 py-2.5"><span class="num">{{ formatNumber(r.count) }}</span></td>
              <td class="px-3 py-2.5"><MoneyText :value="r.sign * r.taxable" plain /></td>
              <td class="px-4 py-2.5"><MoneyText :value="r.sign * r.vat" plain /></td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-if="boxes.length" class="mt-5 overflow-hidden rounded-xl border border-border">
        <div class="border-b border-border bg-surface px-4 py-2.5 text-xs font-medium text-text-secondary">مربعات إقرار ضريبة القيمة المضافة — المبيعات (حسب الفئة الضريبية)</div>
        <table class="w-full text-body">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2.5 text-start font-medium">الفئة</th>
              <th class="px-3 py-2.5 text-start font-medium">النسبة</th>
              <th class="px-3 py-2.5 text-start font-medium">صافي المبيعات</th>
              <th class="px-4 py-2.5 text-start font-medium">الضريبة</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="b in boxes" :key="`${b.category}-${b.rate}`" class="border-b border-border last:border-0">
              <td class="px-4 py-2.5">{{ CATEGORY_LABEL[b.category] ?? b.category }}</td>
              <td class="px-3 py-2.5"><span class="num">{{ b.rate }}%</span></td>
              <td class="px-3 py-2.5"><MoneyText :value="b.net" plain /></td>
              <td class="px-4 py-2.5"><MoneyText :value="b.vat" plain /></td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="mt-5 grid gap-4 sm:grid-cols-3">
        <div class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">ضريبة المخرجات (المبيعات)</p>
          <p class="mt-1 text-lg font-semibold"><MoneyText :value="data.outputVat" /></p>
        </div>
        <div class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">ضريبة المدخلات (المشتريات)</p>
          <p class="mt-1 text-lg font-semibold"><MoneyText :value="data.inputVat" /></p>
        </div>
        <div class="rounded-xl border-2 p-4" :class="data.netPayable >= 0 ? 'border-warning/50' : 'border-success/50'">
          <p class="text-xs text-text-secondary">{{ data.netPayable >= 0 ? 'صافي الضريبة المستحقة للهيئة' : 'رصيد ضريبي مسترد' }}</p>
          <p class="mt-1 text-xl font-semibold"><MoneyText :value="Math.abs(data.netPayable)" /></p>
        </div>
      </div>

      <p class="mt-4 flex items-center gap-1.5 text-xs" :class="reconciled ? 'text-success' : 'text-danger'">
        <CircleCheck v-if="reconciled" class="size-3.5" />
        <CircleAlert v-else class="size-3.5" />
        {{ reconciled ? 'مطابق لأرصدة حسابات الضريبة في دفتر الأستاذ (2150 و 1150)' : 'يوجد فرق مع حسابات الضريبة في دفتر الأستاذ — راجع القيود اليدوية' }}
      </p>
    </template>
  </ReportShell>
</template>
