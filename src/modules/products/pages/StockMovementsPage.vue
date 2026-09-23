<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ArrowLeftRight } from '@lucide/vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { daysAgoKey, formatDateTime, formatNumber, todayKey } from '@/modules/core/helpers/format';
import { MOVEMENT_REASON } from '@/modules/core/helpers/labels';
import { getStockMovements } from '../services/inventoryService';
import { getProducts } from '../services/productService';
import type { Product, StockMovementReason } from '../types';

const route = useRoute();
const router = useRouter();

const productId = ref<string | undefined>(typeof route.query.product === 'string' ? route.query.product : undefined);
const reason = ref<StockMovementReason | ''>('');
const from = ref(daysAgoKey(29));
const to = ref(todayKey());
const products = ref<Product[]>([]);

const movements = useAsync(() =>
  getStockMovements({ productId: productId.value, reason: reason.value || undefined, from: from.value || undefined, to: to.value || undefined }),
);
onMounted(async () => (products.value = await getProducts({ type: 'product', includeInactive: true })));
watch([productId, reason, from, to], () => {
  router.replace({ query: { product: productId.value } });
  movements.reload();
});

const productOptions = computed(() => products.value.map((p) => ({ value: p.id, label: p.name, sublabel: p.sku, keywords: `${p.sku} ${p.barcode ?? ''}` })));
const reasonOptions = (Object.keys(MOVEMENT_REASON) as StockMovementReason[]).map((r) => ({ value: r, label: MOVEMENT_REASON[r] }));

const totals = computed(() => {
  const list = movements.data.value ?? [];
  return {
    in: list.filter((m) => m.qtyChange > 0).reduce((a, m) => a + m.qtyChange, 0),
    out: list.filter((m) => m.qtyChange < 0).reduce((a, m) => a - m.qtyChange, 0),
  };
});

type Row = NonNullable<typeof movements.data.value>[number];
const columns = computed<Column<Row>[]>(() => [
  { key: 'date', label: 'التاريخ', sortable: true },
  ...(productId.value ? [] : [{ key: 'productName', label: 'الصنف', sortable: true }]),
  { key: 'reason', label: 'نوع الحركة', sortable: true },
  { key: 'refNumber', label: 'المستند' },
  { key: 'qtyChange', label: 'الكمية', numeric: true, sortable: true },
  { key: 'balanceAfter', label: 'الرصيد بعد الحركة', numeric: true },
]);
</script>

<template>
  <div>
    <PageHeader title="حركة المخزون" subtitle="سجل كل ما دخل وخرج من المخزون ومصدره — للقراءة فقط" />

    <div class="mb-3 flex flex-wrap items-end justify-between gap-3">
      <div class="flex flex-wrap items-end gap-2">
        <AppCombobox v-model="productId" class="w-64" :options="productOptions" placeholder="كل الأصناف" clearable />
        <AppSelect v-model="reason" class="w-40" placeholder="كل الحركات" :options="reasonOptions" />
      </div>
      <DateRangeFilter v-model:from="from" v-model:to="to" />
    </div>

    <DataTable
      :columns="columns"
      :rows="movements.data.value"
      :loading="movements.loading.value"
      :error="movements.error.value"
      clickable
      :empty-icon="ArrowLeftRight"
      empty-title="لا توجد حركات في هذه الفترة"
      @retry="movements.reload"
      @row-click="(m) => m.refLink && router.push(m.refLink)"
    >
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
      <template #cell-reason="{ row }">{{ MOVEMENT_REASON[row.reason] }}</template>
      <template #cell-refNumber="{ row }"><span class="num text-primary">{{ row.refNumber }}</span></template>
      <template #cell-qtyChange="{ row }">
        <span class="num font-medium" :class="row.qtyChange > 0 ? 'text-success' : 'text-danger'">{{ row.qtyChange > 0 ? '+' : '' }}{{ formatNumber(row.qtyChange) }}</span>
      </template>
      <template #cell-balanceAfter="{ row }"><span class="num">{{ formatNumber(row.balanceAfter) }}</span></template>
    </DataTable>
    <p v-if="movements.data.value?.length" class="mt-3 text-xs text-text-secondary">
      {{ formatNumber(movements.data.value.length) }} حركة · وارد <span class="num text-success">+{{ formatNumber(totals.in) }}</span> · صادر
      <span class="num text-danger">−{{ formatNumber(totals.out) }}</span>
    </p>
  </div>
</template>
