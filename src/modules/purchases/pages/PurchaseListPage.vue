<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { Plus, ShoppingBag } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate, formatNumber } from '@/modules/core/helpers/format';
import { PAYMENT_STATUS, PURCHASE_STATUS } from '@/modules/core/helpers/labels';
import { matchesSearch } from '@/modules/core/helpers/search';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getPurchaseOrders, type PurchaseRow } from '../services/purchaseService';

type View = 'all' | 'DRAFT' | 'CONFIRMED' | 'open' | 'CANCELED';

const router = useRouter();
const auth = useAuthStore();
const view = ref<View>('all');
const search = ref('');
const from = ref('');
const to = ref('');

const { data, loading, error, reload } = useAsync(() => getPurchaseOrders({ from: from.value || undefined, to: to.value || undefined }));
watch([from, to], reload);

const matches = (r: PurchaseRow, v: View) => v === 'all' || (v === 'open' ? r.outstanding > 0 : r.status === v);

const rows = computed(() =>
  (data.value ?? []).filter((r) => matches(r, view.value) && matchesSearch([r.number, r.supplierName], search.value)),
);

const viewOptions = computed(() =>
  (
    [
      ['all', 'الكل'],
      ['DRAFT', 'مسودات'],
      ['CONFIRMED', 'مؤكدة'],
      ['open', 'غير مسددة'],
      ['CANCELED', 'ملغاة'],
    ] as [View, string][]
  ).map(([value, label]) => ({ value, label, count: data.value?.filter((r) => matches(r, value)).length })),
);

const outstandingTotal = computed(() => rows.value.reduce((a, r) => a + r.outstanding, 0));

const columns: Column<PurchaseRow>[] = [
  { key: 'number', label: 'رقم الأمر', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'supplierName', label: 'المورد', sortable: true },
  { key: 'lines', label: 'الأصناف', numeric: true, sortValue: (r) => r.lines.length },
  { key: 'status', label: 'الحالة' },
  { key: 'grandTotal', label: 'الإجمالي', numeric: true, sortable: true },
  { key: 'outstanding', label: 'المتبقي للمورد', numeric: true, sortable: true },
];
</script>

<template>
  <div>
    <PageHeader title="أوامر الشراء" subtitle="شراء البضاعة من الموردين — تأكيد الأمر يُدخل البضاعة للمخزون ويُسجل القيد">
      <template v-if="auth.can('purchases', 'write')" #actions>
        <AppButton variant="primary" :icon="Plus" to="/purchases/new">أمر شراء جديد</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <SegmentedControl v-model="view" :options="viewOptions" />
      <SearchInput v-model="search" placeholder="رقم الأمر أو اسم المورد" />
    </div>
    <div class="mb-3"><DateRangeFilter v-model:from="from" v-model:to="to" /></div>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="ShoppingBag"
      empty-title="لا توجد أوامر شراء"
      @retry="reload"
      @row-click="(r) => router.push(`/purchases/${r.id}`)"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDate(row.date) }}</span></template>
      <template #cell-lines="{ row }"><span class="num text-text-secondary">{{ formatNumber(row.lines.length) }}</span></template>
      <template #cell-status="{ row }">
        <StatusBadge :tone="PURCHASE_STATUS[row.status].tone" :label="PURCHASE_STATUS[row.status].label" />
        <StatusBadge
          v-if="row.status === 'CONFIRMED' && row.paymentStatus !== 'PAID'"
          class="ms-1.5"
          :tone="PAYMENT_STATUS[row.paymentStatus].tone"
          :label="PAYMENT_STATUS[row.paymentStatus].label"
        />
      </template>
      <template #cell-grandTotal="{ row }"><MoneyText :value="row.grandTotal" /></template>
      <template #cell-outstanding="{ row }"><MoneyText :value="row.outstanding" dash-zero /></template>
    </DataTable>
    <p v-if="rows.length" class="mt-3 text-xs text-text-secondary">
      {{ formatNumber(rows.length) }} أمر · المتبقي للموردين <MoneyText :value="outstandingTotal" class="text-text-primary" />
    </p>
  </div>
</template>
