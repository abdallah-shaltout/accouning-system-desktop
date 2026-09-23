<script setup lang="ts">
import { AlertTriangle, ClipboardCheck, PackageMinus, PackagePlus, PackageX } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateLong, formatNumber } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getExpiryReport, getStockCounts } from '@/modules/products/services/inventoryService';
import { getLowStockProducts } from '../../services/dashboardService';

/**
 * v2 (docs/v2/01-personas.md §2 storekeeper, docs/v2/07-products-and-inventory.md §6): a minimal
 * landing page for أمين مخزن — low stock, expiring soon and open counts, plus the actions they use
 * most. The full insight-driven home (recommendations, per-role branch views) is Phase 10's job;
 * this is deliberately simple.
 */
const auth = useAuthStore();

const lowStock = useAsync(() => getLowStockProducts(8));
const expiry = useAsync(getExpiryReport);
const counts = useAsync(getStockCounts);

const greeting = new Date().getHours() < 12 ? 'صباح الخير' : 'مساء الخير';
</script>

<template>
  <div>
    <div class="mb-5 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 class="text-lg font-semibold tracking-tight">{{ greeting }}، {{ auth.user?.name.split(' ')[0] }}</h1>
        <p class="mt-0.5 text-body text-text-secondary">{{ formatDateLong(new Date().toISOString()) }}</p>
      </div>
      <div class="flex flex-wrap gap-2">
        <AppButton :icon="PackagePlus" to="/inventory/adjustments/new?type=STOCK_IN">إدخال مخزون</AppButton>
        <AppButton :icon="PackageMinus" to="/inventory/adjustments/new?type=LOSS">إتلاف / فقد</AppButton>
        <AppButton variant="primary" :icon="ClipboardCheck" to="/inventory/counts/new">جرد جديد</AppButton>
      </div>
    </div>

    <div class="grid gap-4 xl:grid-cols-3">
      <AppCard title="أصناف منخفضة المخزون" padding="none">
        <template v-if="lowStock.data.value?.length" #actions>
          <AppButton size="sm" variant="ghost" to="/products?stock=low">عرض الكل</AppButton>
        </template>
        <ErrorState v-if="lowStock.error.value" :message="lowStock.error.value" compact @retry="lowStock.reload" />
        <div v-else-if="lowStock.loading.value" class="p-4"><SkeletonBlock :lines="5" /></div>
        <EmptyState v-else-if="!lowStock.data.value?.length" title="المخزون بحالة جيدة" compact />
        <ul v-else class="divide-y divide-border">
          <li v-for="p in lowStock.data.value" :key="p.id">
            <RouterLink :to="`/products/${p.id}`" class="flex items-center justify-between gap-3 px-4 py-2.5 hover:bg-surface-hover">
              <span class="min-w-0">
                <span class="block truncate text-body">{{ p.name }}</span>
                <span class="num block text-tiny text-text-secondary">{{ p.sku }}</span>
              </span>
              <span class="shrink-0 text-end text-xs">
                <span class="num block font-medium" :class="p.stockQty <= 0 ? 'text-danger' : 'text-warning'">{{ formatNumber(p.stockQty) }}</span>
                <span class="block text-tiny text-text-secondary">الحد <span class="num">{{ formatNumber(p.minStock) }}</span></span>
              </span>
            </RouterLink>
          </li>
        </ul>
      </AppCard>

      <AppCard title="تنتهي صلاحيتها قريباً" padding="none">
        <template v-if="expiry.data.value?.length" #actions>
          <AppButton size="sm" variant="ghost" to="/inventory/expiry">عرض الكل</AppButton>
        </template>
        <ErrorState v-if="expiry.error.value" :message="expiry.error.value" compact @retry="expiry.reload" />
        <div v-else-if="expiry.loading.value" class="p-4"><SkeletonBlock :lines="5" /></div>
        <EmptyState v-else-if="!expiry.data.value?.length" :icon="PackageX" title="لا توجد أصناف قريبة من الانتهاء" compact />
        <ul v-else class="divide-y divide-border">
          <li v-for="r in expiry.data.value.slice(0, 8)" :key="r.id" class="flex items-center justify-between gap-3 px-4 py-2.5">
            <span class="min-w-0">
              <span class="block truncate text-body">{{ r.productName }}</span>
              <span class="num block text-tiny text-text-secondary">{{ r.batchNo }}</span>
            </span>
            <StatusBadge :tone="r.bucket === 'expired' ? 'danger' : 'warning'" :label="r.bucket === 'expired' ? 'منتهي' : `${r.daysLeft} يوم`" />
          </li>
        </ul>
      </AppCard>

      <AppCard title="عمليات جرد مفتوحة" padding="none">
        <template v-if="counts.data.value?.some((c) => c.status !== 'COMPLETED')" #actions>
          <AppButton size="sm" variant="ghost" to="/inventory/counts">عرض الكل</AppButton>
        </template>
        <ErrorState v-if="counts.error.value" :message="counts.error.value" compact @retry="counts.reload" />
        <div v-else-if="counts.loading.value" class="p-4"><SkeletonBlock :lines="4" /></div>
        <EmptyState v-else-if="!counts.data.value?.some((c) => c.status !== 'COMPLETED')" :icon="AlertTriangle" title="لا توجد عمليات جرد مفتوحة" compact />
        <ul v-else class="divide-y divide-border">
          <template v-for="c in counts.data.value" :key="c.id">
            <li v-if="c.status !== 'COMPLETED'">
              <RouterLink :to="`/inventory/counts/${c.id}`" class="flex items-center justify-between gap-3 px-4 py-2.5 hover:bg-surface-hover">
                <span class="num text-body">{{ c.number }}</span>
                <StatusBadge :tone="c.status === 'REVIEW' ? 'neutral' : 'warning'" :label="c.status === 'REVIEW' ? 'قيد المراجعة' : 'جارٍ العد'" />
              </RouterLink>
            </li>
          </template>
        </ul>
      </AppCard>
    </div>
  </div>
</template>
