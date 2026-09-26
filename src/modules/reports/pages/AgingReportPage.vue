<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "AR aging / AP aging: buckets 0–30/31–60/61–90/90+ by due date; per party, drillable"). */
import { computed, ref, watch } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber, todayKey } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import type { ExportTable } from '../helpers/export';
import { count, kpis, money, row, table as printTable } from '../print/build';
import type { ReportPrintSpec } from '../print/types';
import { getAgingReport } from '../services/reportService';
import type { AgingReportRow } from '../types';

const kind = ref<'customer' | 'supplier'>('customer');
const { data, loading, error, reload } = useAsync(() => getAgingReport(kind.value), { immediate: false });
watch(kind, reload, { immediate: true });

const totals = computed(() => {
  const rows = data.value ?? [];
  return {
    current: rows.reduce((a, r) => a + r.current, 0),
    b30: rows.reduce((a, r) => a + r.b30, 0),
    b60: rows.reduce((a, r) => a + r.b60, 0),
    b90plus: rows.reduce((a, r) => a + r.b90plus, 0),
    total: rows.reduce((a, r) => a + r.total, 0),
  };
});

const insights = computed(() => {
  const t = totals.value;
  if (!data.value?.length) return null;
  const overduePct = t.total > 0 ? ((t.b30 + t.b60 + t.b90plus) / t.total) * 100 : 0;
  return {
    headline: `${formatNumber(t.total)} إجمالي أرصدة ${kind.value === 'customer' ? 'العملاء' : 'الموردين'} المفتوحة`,
    metrics: [
      { label: 'متأخر (بعد تاريخ الاستحقاق)', value: `${formatNumber(overduePct, 1)}%` },
      { label: '90+ يوماً', value: formatNumber(t.b90plus) },
    ],
  };
});

const columns: Column<AgingReportRow>[] = [
  { key: 'name', label: kind.value === 'customer' ? 'العميل' : 'المورد', sortable: true },
  { key: 'current', label: 'حتى الاستحقاق', numeric: true, sortable: true },
  { key: 'b30', label: '1–30 يوم', numeric: true, sortable: true },
  { key: 'b60', label: '31–60 يوم', numeric: true, sortable: true },
  { key: 'b90plus', label: '90+ يوم', numeric: true, sortable: true },
  { key: 'total', label: 'الإجمالي', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && {
    title: kind.value === 'customer' ? 'أعمار ديون العملاء' : 'أعمار ديون الموردين',
    columns: columns.map((c) => c.label),
    rows: [...data.value.map((r) => [r.name, r.current, r.b30, r.b60, r.b90plus, r.total]), ['الإجمالي', totals.value.current, totals.value.b30, totals.value.b60, totals.value.b90plus, totals.value.total]],
  },
);

// Official print (reference `accountsReceivableAging.hbs`: bucket bar, then one row per party).
const print = computed<ReportPrintSpec | null>(() => {
  const rows = data.value;
  if (!rows) return null;
  const t = totals.value;
  const m = (v: number) => money(v, { dashZero: true });
  return {
    meta: [{ label: 'النوع', value: kind.value === 'customer' ? 'ذمم مدينة (العملاء)' : 'ذمم دائنة (الموردون)' }],
    blocks: [
      kpis(columns.slice(1).map((c) => ({ label: c.label, value: money(t[c.key as keyof typeof t]), emphasis: c.key === 'total' }))),
      printTable(
        [{ label: '#', dim: true, align: 'center', width: 0.4 }, { label: kind.value === 'customer' ? 'العميل' : 'المورد', width: 3 }, ...columns.slice(1).map((c) => ({ label: c.label, numeric: true, width: 1.2 }))],
        [
          ...rows.map((r, i) => row([count(i + 1), r.name, m(r.current), m(r.b30), m(r.b60), m(r.b90plus), money(r.total)])),
          row(['', 'الإجمالي', money(t.current), money(t.b30), money(t.b60), money(t.b90plus), money(t.total)], 'total'),
        ],
        'لا توجد أرصدة مفتوحة',
      ),
    ],
  };
});
</script>

<template>
  <ReportShell
    :title="kind === 'customer' ? 'أعمار ديون العملاء' : 'أعمار ديون الموردين'"
    subtitle="الأرصدة المفتوحة موزعة حسب عدد أيام التأخر عن تاريخ الاستحقاق"
    :as-of="todayKey()"
    :loading="loading && !data"
    :error="error"
    :table="table"
    :print="print"
    :insights="insights"
    @retry="reload"
  >
    <template #filters>
      <SegmentedControl v-model="kind" :options="[{ value: 'customer', label: 'العملاء' }, { value: 'supplier', label: 'الموردون' }]" />
    </template>

    <DataTable :columns="columns" :rows="data" row-key="partyId" :page-size="0" clickable @row-click="(r) => $router.push(kind === 'customer' ? { name: 'customer', params: { id: r.partyId } } : { name: 'supplier', params: { id: r.partyId } })">
      <template #cell-current="{ row }"><MoneyText :value="row.current" plain dash-zero /></template>
      <template #cell-b30="{ row }"><MoneyText :value="row.b30" plain dash-zero /></template>
      <template #cell-b60="{ row }"><MoneyText :value="row.b60" plain dash-zero class="text-warning" /></template>
      <template #cell-b90plus="{ row }"><MoneyText :value="row.b90plus" plain dash-zero class="text-danger" /></template>
      <template #cell-total="{ row }"><MoneyText :value="row.total" plain class="font-medium" /></template>
      <template #footer>
        <tr>
          <td class="px-3 py-3">الإجمالي</td>
          <td class="px-3 py-3"><MoneyText :value="totals.current" plain /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.b30" plain /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.b60" plain /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.b90plus" plain /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.total" /></td>
        </tr>
      </template>
    </DataTable>
  </ReportShell>
</template>
