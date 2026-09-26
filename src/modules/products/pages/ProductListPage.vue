<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Package, PackagePlus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import FilterBar from '@/modules/core/components/blocks/FilterBar.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import { matchesSearch } from '@/modules/core/helpers/search';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { getProducts, isLowStock } from '../services/productService';
import type { Product } from '../types';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const catalog = useCatalogStore();
const canWrite = computed(() => auth.can('inventory', 'write'));

// `FilterBar` syncs `q`/`category`/`view` to the URL itself; `stock=low` is a legacy deep-link
// (dashboard "low stock" widget) folded into the same `view` filter key.
const search = computed(() => (typeof route.query.q === 'string' ? route.query.q : ''));
const categoryId = computed(() => (typeof route.query.category === 'string' ? route.query.category : ''));
const view = computed<'all' | 'product' | 'service' | 'low' | 'inactive'>(() => {
  if (route.query.stock === 'low') return 'low';
  const v = route.query.view;
  if (v === 'product' || v === 'service' || v === 'low' || v === 'inactive') return v;
  return 'all';
});

const { data, loading, error, reload } = useAsync(() => getProducts({ includeInactive: true }));
onMounted(() => catalog.load());

const rows = computed(() => {
  return (data.value ?? []).filter((p) => {
    if (view.value === 'inactive' ? p.active : !p.active) return false;
    if (view.value === 'product' && p.type !== 'product') return false;
    if (view.value === 'service' && p.type !== 'service') return false;
    if (view.value === 'low' && !isLowStock(p)) return false;
    if (categoryId.value && p.categoryId !== categoryId.value) return false;
    return matchesSearch([p.name, p.sku, p.barcode], search.value);
  });
});

const viewOptions = [
  { value: 'all', label: 'الكل' },
  { value: 'product', label: 'منتجات' },
  { value: 'service', label: 'خدمات' },
  { value: 'low', label: 'مخزون منخفض' },
  { value: 'inactive', label: 'موقوفة' },
];

const stockValue = computed(() => rows.value.reduce((a, p) => a + (p.type === 'product' ? p.stockQty * p.costPrice : 0), 0));

const columns: Column<Product>[] = [
  { key: 'name', label: 'المنتج', sortable: true },
  { key: 'categoryId', label: 'التصنيف', sortable: true, sortValue: (p) => catalog.categoryName(p.categoryId) },
  { key: 'price', label: 'سعر البيع', numeric: true, sortable: true },
  { key: 'costPrice', label: 'التكلفة', numeric: true, sortable: true },
  { key: 'margin', label: 'الهامش', numeric: true, sortable: true, sortValue: (p) => margin(p) },
  { key: 'stockQty', label: 'المخزون', numeric: true, sortable: true },
];

function margin(p: Product) {
  return p.price > 0 ? ((p.price - p.costPrice) / p.price) * 100 : 0;
}
</script>

<template>
    <ListPage
      title="المنتجات"
      subtitle="الأصناف والخدمات، الأسعار، ومستويات المخزون"
      :primary-action-label="canWrite ? 'منتج جديد' : undefined"
      :primary-action-to="{ name: 'product-new' }"
    >
      <template #actions>
        <AppButton v-if="canWrite" :icon="PackagePlus" :to="{ name: 'adjustment-new', query: { type: 'STOCK_IN' } }">إدخال مخزون</AppButton>
      </template>
      <template #filters>
        <FilterBar
          search-placeholder="الاسم، SKU، أو الباركود"
          :filters="[
            { key: 'view', label: 'العرض', options: viewOptions },
            { key: 'category', label: 'التصنيف', placeholder: 'كل التصنيفات', options: catalog.categories.map((c) => ({ value: c.id, label: c.name })) },
          ]"
        />
      </template>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      export-file-name="المنتجات"
      :empty-icon="Package"
      empty-title="لا توجد منتجات مطابقة"
      empty-description="جرّب تغيير الفلاتر أو البحث بكلمة أخرى"
      @retry="reload"
      @row-click="(p) => router.push({ name: 'product', params: { id: p.id } })"
    >
      <template #cell-name="{ row }">
        <div class="flex items-center gap-2">
          <span class="font-medium">{{ row.name }}</span>
          <StatusBadge v-if="row.type === 'service'" tone="primary" label="خدمة" :dot="false" />
        </div>
        <div class="num mt-0.5 text-xs text-text-secondary">{{ row.sku }}<template v-if="row.barcode"> · {{ row.barcode }}</template></div>
      </template>
      <template #cell-categoryId="{ row }"><span class="text-text-secondary">{{ catalog.categoryName(row.categoryId) }}</span></template>
      <template #cell-price="{ row }"><MoneyText :value="row.price" /></template>
      <template #cell-costPrice="{ row }"><MoneyText :value="row.costPrice" plain class="text-text-secondary" /></template>
      <template #cell-margin="{ row }"><span class="num text-text-secondary">{{ formatNumber(margin(row), 1) }}%</span></template>
      <template #cell-stockQty="{ row }">
        <span v-if="row.type === 'service'" class="text-text-secondary">—</span>
        <span v-else class="inline-flex items-center gap-1.5">
          <span class="num font-medium" :class="row.stockQty <= 0 ? 'text-danger' : isLowStock(row) ? 'text-warning' : ''">{{ formatNumber(row.stockQty) }}</span>
          <span class="text-xs text-text-secondary">{{ catalog.unitName(row.unitId) }}</span>
        </span>
      </template>
    </DataTable>

    <p v-if="rows.length" class="mt-3 text-xs text-text-secondary">
      {{ formatNumber(rows.length) }} صنف · قيمة المخزون بالتكلفة <MoneyText :value="stockValue" />
    </p>
    </ListPage>
</template>
