<script setup lang="ts">
import { computed, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ShoppingBag } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import FilterBar from '@/modules/core/components/blocks/FilterBar.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import { PAYMENT_STATUS, PURCHASE_STATUS } from '@/modules/core/helpers/labels';
import { matchesSearch } from '@/modules/core/helpers/search';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getPurchaseOrders, type PurchaseRow } from '../services/purchaseService';

type View = 'all' | 'DRAFT' | 'ORDERED' | 'RECEIVED' | 'open' | 'CANCELED';

const router = useRouter();
const route = useRoute();
const auth = useAuthStore();

const view = computed<View>(() => (typeof route.query.view === 'string' ? (route.query.view as View) : 'all'));
const search = computed(() => (typeof route.query.q === 'string' ? route.query.q : ''));
const from = computed(() => (typeof route.query.from === 'string' ? route.query.from : ''));
const to = computed(() => (typeof route.query.to === 'string' ? route.query.to : ''));

const { data, loading, error, reload } = useAsync(() => getPurchaseOrders({ from: from.value || undefined, to: to.value || undefined }));
watch([from, to], reload);

const matches = (r: PurchaseRow, v: View) => v === 'all' || (v === 'open' ? r.outstanding > 0 : r.status === v);

const rows = computed(() =>
  (data.value ?? []).filter((r) => matches(r, view.value) && matchesSearch([r.number, r.supplierName], search.value)),
);

const viewOptions = [
  { value: 'all', label: 'الكل' },
  { value: 'DRAFT', label: 'مسودات' },
  { value: 'ORDERED', label: 'مرسلة' },
  { value: 'RECEIVED', label: 'مستلمة' },
  { value: 'open', label: 'غير مسددة' },
  { value: 'CANCELED', label: 'ملغاة' },
];

const outstandingTotal = computed(() => rows.value.reduce((a, r) => a + r.outstanding, 0));

const columns: Column<PurchaseRow>[] = [
  { key: 'number', label: 'رقم الأمر', sortable: true },
  { key: 'date', label: 'التاريخ', type: 'date', sortable: true },
  { key: 'supplierName', label: 'المورد', type: 'party', sortable: true },
  { key: 'lines', label: 'الأصناف', numeric: true, sortValue: (r) => r.lines.length },
  { key: 'status', label: 'الحالة' },
  { key: 'grandTotal', label: 'الإجمالي', type: 'money', sortable: true },
  { key: 'outstanding', label: 'المتبقي للمورد', type: 'money', sortable: true },
];
</script>

<template>
  <ListPage
    title="أوامر الشراء"
    subtitle="شراء البضاعة من الموردين — تأكيد الأمر يُدخل البضاعة للمخزون ويُسجل القيد"
    :primary-action-label="auth.can('purchases', 'write') ? 'أمر شراء جديد' : undefined"
    :primary-action-to="{ name: 'purchase-new' }"
  >
    <template #filters>
      <FilterBar search-placeholder="رقم الأمر أو اسم المورد" :filters="[{ key: 'view', label: 'الحالة', options: viewOptions }]" date-range />
    </template>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="ShoppingBag"
      empty-title="لا توجد أوامر شراء"
      @retry="reload"
      @row-click="(r) => router.push({ name: 'purchase', params: { id: r.id } })"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-lines="{ row }"><span class="num text-text-secondary">{{ formatNumber(row.lines.length) }}</span></template>
      <template #cell-status="{ row }">
        <StatusBadge :tone="PURCHASE_STATUS[row.status].tone" :label="PURCHASE_STATUS[row.status].label" />
        <StatusBadge
          v-if="row.status === 'RECEIVED' && row.paymentStatus !== 'PAID'"
          class="ms-1.5"
          :tone="PAYMENT_STATUS[row.paymentStatus].tone"
          :label="PAYMENT_STATUS[row.paymentStatus].label"
        />
      </template>
      <template #cell-outstanding="{ row }"><MoneyText :value="row.outstanding" dash-zero /></template>
    </DataTable>
    <p v-if="rows.length" class="mt-3 text-xs text-text-secondary">
      {{ formatNumber(rows.length) }} أمر · المتبقي للموردين <MoneyText :value="outstandingTotal" class="text-text-primary" />
    </p>
  </ListPage>
</template>
