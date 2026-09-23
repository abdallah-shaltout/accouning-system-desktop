<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ArrowLeftRight, PackagePlus, Pencil } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { MOVEMENT_REASON } from '@/modules/core/helpers/labels';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { getStockMovements } from '../services/inventoryService';
import { getProduct, isLowStock } from '../services/productService';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const catalog = useCatalogStore();
const id = String(route.params.id);

const product = useAsync(() => getProduct(id));
const movements = useAsync(() => getStockMovements({ productId: id }));
onMounted(() => catalog.load());

const p = computed(() => product.data.value);
const soldLast30 = computed(() => {
  const since = Date.now() - 30 * 86400_000;
  return -(movements.data.value ?? [])
    .filter((m) => (m.reason === 'sale' || m.reason === 'refund') && new Date(m.date).getTime() >= since)
    .reduce((a, m) => a + m.qtyChange, 0);
});

type Movement = NonNullable<typeof movements.data.value>[number];
const columns: Column<Movement>[] = [
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'reason', label: 'الحركة' },
  { key: 'refNumber', label: 'المستند' },
  { key: 'qtyChange', label: 'الكمية', numeric: true },
  { key: 'balanceAfter', label: 'الرصيد بعد الحركة', numeric: true },
];
</script>

<template>
  <div>
    <ErrorState v-if="product.error.value" :message="product.error.value" @retry="product.reload" />
    <template v-else>
      <PageHeader :title="p?.name ?? '…'" back="/products">
        <template #badge>
          <StatusBadge v-if="p && !p.active" label="موقوف" />
          <StatusBadge v-if="p?.type === 'service'" tone="primary" label="خدمة" :dot="false" />
        </template>
        <template #subtitle>
          <span class="num">{{ p?.sku }}</span><template v-if="p?.barcode"> · <span class="num">{{ p.barcode }}</span></template>
          · {{ catalog.categoryName(p?.categoryId) }}
        </template>
        <template v-if="auth.can('inventory', 'write')" #actions>
          <AppButton v-if="p?.type === 'product'" :icon="PackagePlus" :to="`/inventory/adjustments/new?type=STOCK_IN&product=${id}`">إدخال مخزون</AppButton>
          <AppButton variant="primary" :icon="Pencil" :to="`/products/${id}/edit`">تعديل</AppButton>
        </template>
      </PageHeader>

      <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <AppCard padding="sm">
          <p class="text-xs text-text-secondary">سعر البيع</p>
          <SkeletonBlock v-if="!p" class="mt-2" height="h-6" />
          <p v-else class="mt-1 text-lg font-semibold"><MoneyText :value="p.price" /></p>
          <p v-if="p" class="text-xs text-text-secondary">التكلفة <MoneyText :value="p.costPrice" plain /></p>
        </AppCard>
        <AppCard padding="sm">
          <p class="text-xs text-text-secondary">الرصيد الحالي</p>
          <SkeletonBlock v-if="!p" class="mt-2" height="h-6" />
          <template v-else-if="p.type === 'product'">
            <p class="mt-1 text-lg font-semibold" :class="p.stockQty <= 0 ? 'text-danger' : isLowStock(p) ? 'text-warning' : ''">
              <span class="num">{{ formatNumber(p.stockQty) }}</span> <span class="text-sm font-normal text-text-secondary">{{ catalog.unitName(p.unitId) }}</span>
            </p>
            <p class="text-xs text-text-secondary">الحد الأدنى <span class="num">{{ formatNumber(p.minStock) }}</span></p>
          </template>
          <p v-else class="mt-1 text-sm text-text-secondary">خدمة — بدون مخزون</p>
        </AppCard>
        <AppCard padding="sm">
          <p class="text-xs text-text-secondary">قيمة المخزون (بالتكلفة)</p>
          <SkeletonBlock v-if="!p" class="mt-2" height="h-6" />
          <p v-else class="mt-1 text-lg font-semibold"><MoneyText :value="p.type === 'product' ? p.stockQty * p.costPrice : 0" /></p>
        </AppCard>
        <AppCard padding="sm">
          <p class="text-xs text-text-secondary">المباع آخر 30 يوماً</p>
          <SkeletonBlock v-if="movements.loading.value" class="mt-2" height="h-6" />
          <p v-else class="mt-1 text-lg font-semibold"><span class="num">{{ formatNumber(soldLast30) }}</span></p>
        </AppCard>
      </div>

      <div class="mt-5 grid items-start gap-5 xl:grid-cols-[1fr_300px]">
        <div>
          <h2 class="mb-2 flex items-center gap-2 text-[13px] font-semibold"><ArrowLeftRight class="size-4 text-text-secondary" /> حركة المخزون</h2>
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
        </div>

        <AppCard title="قوائم الأسعار" padding="none">
          <ul class="divide-y divide-border text-[13px]">
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
      </div>
    </template>
  </div>
</template>
