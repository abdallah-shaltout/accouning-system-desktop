<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §2 "Quotations (عروض الأسعار)"). */
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { FileText } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import FilterBar from '@/modules/core/components/blocks/FilterBar.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime } from '@/modules/core/helpers/format';
import { getQuotations, type QuotationRow } from '../services/invoiceService';
import type { QuotationStatus } from '../types';

const router = useRouter();
const route = useRoute();
const search = computed(() => (typeof route.query.q === 'string' ? route.query.q : ''));
const { data, loading, error, reload } = useAsync(() => getQuotations());
const rows = computed(() =>
  (data.value ?? []).filter((r) => !search.value || r.number.includes(search.value) || r.customerName?.includes(search.value)),
);

const STATUS_LABEL: Record<QuotationStatus, { label: string; tone: 'neutral' | 'primary' | 'success' | 'danger' | 'warning' }> = {
  DRAFT: { label: 'مسودة', tone: 'neutral' },
  SENT: { label: 'مُرسل', tone: 'primary' },
  ACCEPTED: { label: 'مقبول', tone: 'success' },
  REJECTED: { label: 'مرفوض', tone: 'danger' },
  EXPIRED: { label: 'منتهي', tone: 'warning' },
};

const columns: Column<QuotationRow>[] = [
  { key: 'number', label: 'الرقم', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'customerName', label: 'العميل', sortable: true, sortValue: (r) => r.customerName ?? '' },
  { key: 'status', label: 'الحالة' },
  { key: 'grandTotal', label: 'الإجمالي', numeric: true, sortable: true },
];
</script>

<template>
  <ListPage title="عروض الأسعار" subtitle="عروض أسعار قابلة للتحويل إلى فاتورة" primary-action-label="عرض سعر جديد" :primary-action-to="{ name: 'quotation-new' }">
    <template #filters>
      <FilterBar search-placeholder="رقم العرض أو اسم العميل" />
    </template>
    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="FileText"
      empty-title="لا توجد عروض أسعار"
      @retry="reload"
      @row-click="(r) => router.push({ name: 'quotation', params: { id: r.id } })"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
      <template #cell-customerName="{ row }"><span :class="!row.customerName && 'text-text-secondary'">{{ row.customerName ?? '—' }}</span></template>
      <template #cell-status="{ row }"><StatusBadge :tone="STATUS_LABEL[row.status].tone" :label="STATUS_LABEL[row.status].label" /></template>
      <template #cell-grandTotal="{ row }"><MoneyText :value="row.grandTotal" /></template>
    </DataTable>
  </ListPage>
</template>
