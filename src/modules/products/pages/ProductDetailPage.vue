<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { PackagePlus, Pencil, Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import DetailPage, { type DetailTab } from '@/modules/core/components/layouts/DetailPage.vue';
import type { MetaChip } from '@/modules/core/components/blocks/DetailHeader.vue';
import type { StatCard } from '@/modules/core/components/blocks/StatCards.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate, formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { MOVEMENT_REASON } from '@/modules/core/helpers/labels';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import ProductInsightHints from '@/modules/core/components/insights/ProductInsightHints.vue';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { batchAlertTone, getBatches, getStockMovements } from '../services/inventoryService';
import { getProduct } from '../services/productService';

const route = useRoute('product');
const router = useRouter();
const auth = useAuthStore();
const catalog = useCatalogStore();
const id = String(route.params.id);

const product = useAsync(() => getProduct(id));
const movements = useAsync(() => getStockMovements({ productId: id }));
const batches = useAsync(() => getBatches(id));
onMounted(() => catalog.load());

const p = computed(() => product.data.value);
const soldLast30 = computed(() => {
  const since = Date.now() - 30 * 86400_000;
  return -(movements.data.value ?? [])
    .filter((m) => (m.reason === 'sale' || m.reason === 'refund') && new Date(m.date).getTime() >= since)
    .reduce((a, m) => a + m.qtyChange, 0);
});

const chips = computed<MetaChip[]>(() => {
  if (!p.value) return [];
  const c: MetaChip[] = [{ label: 'الكود', value: p.value.sku }];
  if (p.value.barcode) c.push({ label: 'الباركود', value: p.value.barcode });
  c.push({ label: 'التصنيف', value: catalog.categoryName(p.value.categoryId) ?? '—' });
  return c;
});

const stats = computed<StatCard[]>(() => {
  if (!p.value) return [];
  const s: StatCard[] = [
    { label: 'سعر البيع', value: formatNumber(p.value.price) },
  ];
  if (p.value.type === 'product') {
    s.push({ label: 'الرصيد الحالي', value: `${formatNumber(p.value.stockQty)} ${catalog.unitName(p.value.unitId) ?? ''}` });
    s.push({ label: 'قيمة المخزون (بالتكلفة)', value: formatNumber(p.value.stockQty * p.value.costPrice) });
  } else {
    s.push({ label: 'الرصيد الحالي', value: 'خدمة — بدون مخزون' });
  }
  s.push({ label: 'المباع آخر 30 يوماً', value: formatNumber(soldLast30.value) });
  return s;
});

const tabs = computed<DetailTab[]>(() => {
  const t: DetailTab[] = [{ key: 'movements', label: 'حركة المخزون' }];
  if (p.value?.trackBatches) t.push({ key: 'batches', label: 'التشغيلات' });
  return t;
});

type Movement = NonNullable<typeof movements.data.value>[number];
const columns: Column<Movement>[] = [
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'reason', label: 'الحركة' },
  { key: 'refNumber', label: 'المستند' },
  { key: 'qtyChange', label: 'الكمية', numeric: true },
  { key: 'balanceAfter', label: 'الرصيد بعد الحركة', numeric: true },
];

type Batch = NonNullable<typeof batches.data.value>[number];
const batchColumns: Column<Batch>[] = [
  { key: 'batchNo', label: 'رقم التشغيلة' },
  { key: 'expiryDate', label: 'تاريخ الصلاحية' },
  { key: 'qty', label: 'الكمية المتبقية', type: 'number' },
  { key: 'unitCost', label: 'التكلفة', type: 'money' },
];
</script>

<template>
  <div>
    <ErrorState v-if="product.error.value" :message="product.error.value" @retry="product.reload" />
    <DetailPage
      v-else
      :title="p?.name ?? '…'"
      :status="p && !p.active ? { label: 'موقوف', tone: 'neutral' } : (p?.type === 'service' ? { label: 'خدمة', tone: 'primary' } : undefined)"
      :chips="chips"
      :back="{ name: 'products' }"
      :stats="stats"
      :tabs="tabs"
    >
      <template #actions>
        <!-- v2 phase 11b (docs/v2/07-products-and-inventory.md §6 "The product page: طباعة ملصقات"). -->
        <AppButton :icon="Printer" :to="{ name: 'labels', query: { productId: id } }">طباعة ملصقات</AppButton>
        <template v-if="auth.can('inventory', 'write')">
          <AppButton v-if="p?.type === 'product'" :icon="PackagePlus" :to="{ name: 'adjustment-new', query: { type: 'STOCK_IN', product: id } }">إدخال مخزون</AppButton>
          <AppButton variant="primary" :icon="Pencil" :to="{ name: 'product-edit', params: { id } }">تعديل</AppButton>
        </template>
      </template>

      <template #tab-movements>
        <ProductInsightHints :product-id="id" class="mb-4" />
        <DataTable
          :columns="columns"
          :rows="p?.type === 'service' ? [] : movements.data.value"
          :loading="movements.loading.value"
          :error="movements.error.value"
          :page-size="15"
          clickable
          :empty-title="p?.type === 'service' ? 'الخدمات لا يُتتبع لها مخزون' : 'لا توجد حركات'"
          @retry="movements.reload"
          @row-click="(m) => m.refLink && router.push(m.refLink)"
        >
          <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
          <template #cell-reason="{ row }">{{ MOVEMENT_REASON[row.reason] }}</template>
          <template #cell-refNumber="{ row }"><span class="num text-primary">{{ row.refNumber }}</span></template>
          <template #cell-qtyChange="{ row }">
            <span class="num font-medium" :class="row.qtyChange > 0 ? 'text-success' : 'text-danger'">
              {{ row.qtyChange > 0 ? '+' : '' }}{{ formatNumber(row.qtyChange) }}
            </span>
          </template>
          <template #cell-balanceAfter="{ row }"><span class="num">{{ formatNumber(row.balanceAfter) }}</span></template>
        </DataTable>
      </template>

      <template v-if="p?.trackBatches" #tab-batches>
        <AppCard padding="none">
          <div v-if="batches.loading.value" class="p-4"><SkeletonBlock :lines="4" /></div>
          <DataTable v-else :columns="batchColumns" :rows="batches.data.value" row-key="id" empty-title="لا توجد تشغيلات حالياً">
            <template #cell-batchNo="{ row }"><span class="num">{{ row.batchNo }}</span></template>
            <template #cell-expiryDate="{ row }">
              <span
                class="num inline-flex items-center gap-1.5"
                :class="{ 'text-danger': batchAlertTone(row, p?.expiryAlertDays ?? 30) === 'danger', 'text-warning': batchAlertTone(row, p?.expiryAlertDays ?? 30) === 'warning' }"
              >
                <span v-if="batchAlertTone(row, p?.expiryAlertDays ?? 30)" class="size-1.5 rounded-full bg-current" />
                {{ formatDate(row.expiryDate) }}
              </span>
            </template>
          </DataTable>
        </AppCard>
      </template>

      <template #aside>
        <AppCard title="قوائم الأسعار" padding="none">
          <ul class="divide-y divide-border text-body">
            <li class="flex items-center justify-between px-4 py-2.5">
              <span>السعر الأساسي</span>
              <MoneyText :value="p?.price" />
            </li>
            <li v-for="pl in catalog.priceLists" :key="pl.id" class="flex items-center justify-between px-4 py-2.5">
              <span :class="!pl.active && 'text-text-secondary'">{{ pl.name }}</span>
              <MoneyText v-if="p?.prices?.find((x) => x.priceListId === pl.id)" :value="p.prices.find((x) => x.priceListId === pl.id)!.value" />
              <span v-else class="text-xs text-text-secondary">= الأساسي</span>
            </li>
          </ul>
        </AppCard>
      </template>
    </DetailPage>
  </div>
</template>
