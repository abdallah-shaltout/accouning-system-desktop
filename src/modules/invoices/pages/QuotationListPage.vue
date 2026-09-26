<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §2 "Quotations (عروض الأسعار)"). */
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { FileText, Plus } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import FilterBar from '@/modules/core/components/blocks/FilterBar.vue';
import { matchesSearch } from '@/modules/core/helpers/search';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { getQuotations, type QuotationRow } from '../services/invoiceService';
import type { QuotationStatus } from '../types';

const router = useRouter();
const route = useRoute();
const { data, loading, error, reload } = useAsync(() => getQuotations());

const STATUS_LABEL: Record<QuotationStatus, { label: string; tone: 'neutral' | 'primary' | 'success' | 'danger' | 'warning' }> = {
  DRAFT: { label: 'مسودة', tone: 'neutral' },
  SENT: { label: 'مُرسل', tone: 'primary' },
  ACCEPTED: { label: 'مقبول', tone: 'success' },
  REJECTED: { label: 'مرفوض', tone: 'danger' },
  EXPIRED: { label: 'منتهي', tone: 'warning' },
};

const rows = computed(() => {
  const search = typeof route.query.q === 'string' ? route.query.q : '';
  return (data.value ?? []).filter((r) => matchesSearch([r.number, r.customerName], search));
});

const columns: Column<QuotationRow>[] = [
  { key: 'number', label: 'الرقم', sortable: true },
  { key: 'date', label: 'التاريخ', type: 'date', sortable: true },
  { key: 'customerName', label: 'العميل', sortable: true, sortValue: (r) => r.customerName ?? '' },
  { key: 'status', label: 'الحالة', type: 'status', statusOf: (v: QuotationStatus) => STATUS_LABEL[v] },
  { key: 'grandTotal', label: 'الإجمالي', type: 'money', sortable: true },
];
</script>

<template>
  <ListPage title="عروض الأسعار" subtitle="عروض أسعار قابلة للتحويل إلى فاتورة" primary-action-label="عرض سعر جديد" :primary-action-icon="Plus" primary-action-to="/sales/quotations/new">
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
      @row-click="(r) => router.push(`/sales/quotations/${r.id}`)"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-customerName="{ row }"><span :class="!row.customerName && 'text-text-secondary'">{{ row.customerName ?? '—' }}</span></template>
    </DataTable>
  </ListPage>
</template>
