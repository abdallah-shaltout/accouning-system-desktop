<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Package, PackagePlus, Plus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
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

const search = ref(String(route.query.q ?? ''));
const categoryId = ref<string>(String(route.query.category ?? ''));
const view = ref<'all' | 'product' | 'service' | 'low' | 'inactive'>(route.query.stock === 'low' ? 'low' : 'all');

const { data, loading, error, reload } = useAsync(() => getProducts({ includeInactive: true }));
onMounted(() => catalog.load());

// Keep filters in the URL so back/forward and dashboard deep links work.
watch([search, categoryId, view], () => {
  router.replace({
    query: {
      q: search.value || undefined,
      category: categoryId.value || undefined,
      stock: view.value === 'low' ? 'low' : undefined,
      view: !['all', 'low'].includes(view.value) ? view.value : undefined,
    },
  });
});
if (route.query.view && ['product', 'service', 'inactive'].includes(String(route.query.view))) view.value = route.query.view as typeof view.value;

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

const counts = computed(() => {
  const list = data.value ?? [];
  return {
    all: list.filter((p) => p.active).length,
    product: list.filter((p) => p.active && p.type === 'product').length,
    service: list.filter((p) => p.active && p.type === 'service').length,
    low: list.filter((p) => p.active && isLowStock(p)).length,
    inactive: list.filter((p) => !p.active).length,
  };
});

const viewOptions = computed(() => [
  { value: 'all' as const, label: 'الكل', count: counts.value.all },
  { value: 'product' as const, label: 'منتجات', count: counts.value.product },
  { value: 'service' as const, label: 'خدمات', count: counts.value.service },
  { value: 'low' as const, label: 'مخزون منخفض', count: counts.value.low },
  { value: 'inactive' as const, label: 'موقوفة', count: counts.value.inactive },
]);

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
  <div>
    <PageHeader title="المنتجات" subtitle="الأصناف والخدمات، الأسعار، ومستويات المخزون">
      <template #actions>
        <AppButton v-if="canWrite" :icon="PackagePlus" :to="{ name: 'adjustment-new', query: { type: 'STOCK_IN' } }">إدخال مخزون</AppButton>
        <AppButton v-if="canWrite" variant="primary" :icon="Plus" :to="{ name: 'product-new' }">منتج جديد</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <SegmentedControl v-model="view" :options="viewOptions" />
      <div class="flex flex-wrap items-center gap-2">
        <AppSelect
          v-model="categoryId"
          class="w-44"
          placeholder="كل التصنيفات"
          :options="catalog.categories.map((c) => ({ value: c.id, label: c.name }))"
        />
        <SearchInput v-model="search" placeholder="الاسم، SKU، أو الباركود" />
      </div>
    </div>

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
  </div>
</template>
