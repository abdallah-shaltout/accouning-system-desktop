<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §2 "Quotations (عروض الأسعار)"). */
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { FileText, Plus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime } from '@/modules/core/helpers/format';
import { getQuotations, type QuotationRow } from '../services/invoiceService';
import type { QuotationStatus } from '../types';

const router = useRouter();
const search = ref('');
const { data, loading, error, reload } = useAsync(() => getQuotations());
const rows = computed(() => data.value ?? []);

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
  <div>
    <PageHeader title="عروض الأسعار" subtitle="عروض أسعار قابلة للتحويل إلى فاتورة">
      <template #actions>
        <AppButton variant="primary" :icon="Plus" :to="{ name: 'quotation-new' }">عرض سعر جديد</AppButton>
      </template>
    </PageHeader>
    <div class="mb-3"><SearchInput v-model="search" placeholder="رقم العرض أو اسم العميل" /></div>
    <DataTable
      :columns="columns"
      :rows="rows.filter((r) => !search || r.number.includes(search) || r.customerName?.includes(search))"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="FileText"
      empty-title="لا توجد عروض أسعار"
      @retry="reload"
      @row-click="(r) => router.push(`/sales/quotations/${r.id}`)"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
      <template #cell-customerName="{ row }"><span :class="!row.customerName && 'text-text-secondary'">{{ row.customerName ?? '—' }}</span></template>
      <template #cell-status="{ row }"><StatusBadge :tone="STATUS_LABEL[row.status].tone" :label="STATUS_LABEL[row.status].label" /></template>
      <template #cell-grandTotal="{ row }"><MoneyText :value="row.grandTotal" /></template>
    </DataTable>
  </div>
</template>
