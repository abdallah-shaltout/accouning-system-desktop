<script setup lang="ts">
import { computed } from 'vue';
import { useRouter } from 'vue-router';
import { Banknote, PackageX, Plus, ReceiptText, ShoppingCart, TrendingUp } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { invoiceOutstanding } from '@/modules/invoices/helpers/totals';
import AppButton from '../components/ui/AppButton.vue';
import AppCard from '../components/ui/AppCard.vue';
import EmptyState from '../components/ui/EmptyState.vue';
import ErrorState from '../components/ui/ErrorState.vue';
import MoneyText from '../components/ui/MoneyText.vue';
import RiyalIcon from '../components/ui/RiyalIcon.vue';
import SkeletonBlock from '../components/ui/SkeletonBlock.vue';
import StatusBadge from '../components/ui/StatusBadge.vue';
import KpiCard from '../components/dashboard/KpiCard.vue';
import SalesTrendChart from '../components/dashboard/SalesTrendChart.vue';
import { useAsync } from '../controllers/useAsync';
import { formatDateLong, formatMoney, formatNumber, formatRelative, formatTime } from '../helpers/format';
import { INVOICE_STATUS, PAYMENT_STATUS } from '../helpers/labels';
import { getDashboardSummary, getLowStockProducts, getRecentActivity, getRecentInvoices } from '../services/dashboardService';

const auth = useAuthStore();
const router = useRouter();

const summary = useAsync(getDashboardSummary);
const recent = useAsync(() => getRecentInvoices(8));
const activity = useAsync(() => getRecentActivity(10));
const lowStock = useAsync(() => getLowStockProducts(6));

const canSeeCash = computed(() => auth.can('accounting'));
const trendTotal = computed(() => summary.data.value?.salesTrend.reduce((a, d) => a + d.total, 0) ?? 0);

const greeting = computed(() => {
  const h = new Date().getHours();
  return h < 12 ? 'صباح الخير' : 'مساء الخير';
});

function retryAll() {
  summary.reload();
  recent.reload();
  activity.reload();
  lowStock.reload();
}
</script>

<template>
  <div>
    <div class="mb-5 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 class="text-lg font-semibold tracking-tight">{{ greeting }}، {{ auth.user?.name.split(' ')[0] }}</h1>
        <p class="mt-0.5 text-[13px] text-text-secondary">{{ formatDateLong(new Date().toISOString()) }}</p>
      </div>
      <div class="flex gap-2">
        <AppButton v-if="auth.can('purchases', 'write')" :icon="Plus" to="/purchases/new">أمر شراء</AppButton>
        <AppButton v-if="auth.can('pos', 'write')" variant="primary" :icon="ShoppingCart" to="/pos">بيع جديد</AppButton>
      </div>
    </div>

    <ErrorState v-if="summary.error.value" :message="summary.error.value" @retry="retryAll" />

    <template v-else>
      <!-- KPI row -->
      <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <KpiCard label="مبيعات اليوم" :icon="TrendingUp" :loading="summary.loading.value" :to="auth.can('sales') ? '/invoices' : undefined">
          <span dir="ltr">{{ formatMoney(summary.data.value?.todaySales) }}</span> <RiyalIcon class="text-[0.7em] text-text-secondary" />
          <template #hint>{{ formatNumber(summary.data.value?.todayInvoiceCount) }} فاتورة — شامل الضريبة وبعد المرتجعات</template>
        </KpiCard>
        <KpiCard
          label="فواتير غير مسددة"
          :icon="ReceiptText"
          :loading="summary.loading.value"
          :to="auth.can('sales') ? { path: '/invoices', query: { payment: 'open' } } : undefined"
        >
          {{ formatNumber(summary.data.value?.unpaidInvoiceCount) }}
          <template #hint>مستحق من العملاء: <MoneyText :value="summary.data.value?.unpaidInvoiceTotal" /></template>
        </KpiCard>
        <KpiCard
          label="أصناف منخفضة المخزون"
          :icon="PackageX"
          :loading="summary.loading.value"
          :tone="(summary.data.value?.lowStockCount ?? 0) > 0 ? 'warning' : undefined"
          :to="{ path: '/products', query: { stock: 'low' } }"
        >
          {{ formatNumber(summary.data.value?.lowStockCount) }}
          <template #hint>عند الحد الأدنى أو أقل</template>
        </KpiCard>
        <KpiCard v-if="canSeeCash" label="النقدية المتاحة" :icon="Banknote" :loading="summary.loading.value" to="/reports/ledger?account=acc-1110">
          <span dir="ltr">{{ formatMoney(summary.data.value?.cashPosition) }}</span> <RiyalIcon class="text-[0.7em] text-text-secondary" />
          <template #hint>
            الصندوق <MoneyText :value="summary.data.value?.cashOnHand" plain /> · البنك <MoneyText :value="summary.data.value?.bankBalance" plain />
          </template>
        </KpiCard>
      </div>

      <!-- Chart + activity -->
      <div class="mt-4 grid gap-4 xl:grid-cols-3">
        <AppCard class="xl:col-span-2" title="صافي المبيعات — آخر 14 يوماً" padding="sm">
          <template #actions>
            <span class="text-xs text-text-secondary">الإجمالي <MoneyText :value="trendTotal" class="text-text-primary" /></span>
          </template>
          <SkeletonBlock v-if="summary.loading.value" height="h-48" />
          <SalesTrendChart v-else-if="summary.data.value" :data="summary.data.value.salesTrend" />
        </AppCard>

        <AppCard title="آخر النشاطات" padding="none">
          <div v-if="activity.loading.value" class="p-4"><SkeletonBlock :lines="6" /></div>
          <EmptyState v-else-if="!activity.data.value?.length" title="لا توجد نشاطات" compact />
          <ul v-else class="max-h-[248px] divide-y divide-border overflow-y-auto">
            <li v-for="a in activity.data.value" :key="a.id">
              <component
                :is="a.link ? 'RouterLink' : 'div'"
                :to="a.link"
                class="flex items-start gap-2.5 px-4 py-2.5"
                :class="a.link && 'hover:bg-surface-hover'"
              >
                <span class="mt-1.5 size-1.5 shrink-0 rounded-full bg-text-secondary/50" />
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-[13px]">{{ a.message }}</span>
                  <span class="block text-[11px] text-text-secondary">{{ a.userName }} · {{ formatRelative(a.date) }}</span>
                </span>
              </component>
            </li>
          </ul>
        </AppCard>
      </div>

      <!-- Recent invoices + low stock -->
      <div class="mt-4 grid gap-4 xl:grid-cols-3">
        <AppCard class="xl:col-span-2" title="أحدث الفواتير" padding="none">
          <template v-if="auth.can('sales')" #actions>
            <AppButton size="sm" variant="ghost" to="/invoices">عرض الكل</AppButton>
          </template>
          <div v-if="recent.loading.value" class="p-4"><SkeletonBlock :lines="6" /></div>
          <EmptyState v-else-if="!recent.data.value?.length" title="لا توجد فواتير بعد" compact />
          <table v-else class="w-full text-[13px]">
            <tbody>
              <tr
                v-for="inv in recent.data.value"
                :key="inv.id"
                class="cursor-pointer border-b border-border last:border-0 hover:bg-surface-hover"
                @click="router.push(`/invoices/${inv.id}`)"
              >
                <td class="px-4 py-2.5"><span class="num font-medium">{{ inv.number }}</span></td>
                <td class="px-2 py-2.5 text-text-secondary">{{ inv.customerName ?? 'عميل نقدي' }}</td>
                <td class="px-2 py-2.5 text-text-secondary"><span class="num">{{ formatTime(inv.date) }}</span></td>
                <td class="px-2 py-2.5">
                  <StatusBadge
                    v-if="inv.status === 'REFUNDED'"
                    :tone="INVOICE_STATUS.REFUNDED.tone"
                    :label="INVOICE_STATUS.REFUNDED.label"
                  />
                  <StatusBadge v-else :tone="PAYMENT_STATUS[inv.paymentStatus].tone" :label="PAYMENT_STATUS[inv.paymentStatus].label" />
                </td>
                <td class="px-4 py-2.5 text-end">
                  <MoneyText :value="inv.grandTotal" />
                  <span v-if="invoiceOutstanding(inv) > 0 && inv.status !== 'REFUNDED'" class="block text-[11px] text-warning">
                    متبقي <MoneyText :value="invoiceOutstanding(inv)" plain />
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </AppCard>

        <AppCard title="تنبيهات المخزون" padding="none">
          <template v-if="auth.can('purchases', 'write') && lowStock.data.value?.length" #actions>
            <AppButton size="sm" variant="ghost" to="/purchases/new">طلب شراء</AppButton>
          </template>
          <div v-if="lowStock.loading.value" class="p-4"><SkeletonBlock :lines="5" /></div>
          <EmptyState v-else-if="!lowStock.data.value?.length" title="المخزون بحالة جيدة" description="لا توجد أصناف عند الحد الأدنى" compact />
          <ul v-else class="divide-y divide-border">
            <li v-for="p in lowStock.data.value" :key="p.id">
              <RouterLink :to="`/products/${p.id}`" class="flex items-center justify-between gap-3 px-4 py-2.5 hover:bg-surface-hover">
                <span class="min-w-0">
                  <span class="block truncate text-[13px]">{{ p.name }}</span>
                  <span class="num block text-[11px] text-text-secondary">{{ p.sku }}</span>
                </span>
                <span class="shrink-0 text-end text-xs">
                  <span class="num block font-medium" :class="p.stockQty <= 0 ? 'text-danger' : 'text-warning'">{{ formatNumber(p.stockQty) }}</span>
                  <span class="block text-[11px] text-text-secondary">الحد <span class="num">{{ formatNumber(p.minStock) }}</span></span>
                </span>
              </RouterLink>
            </li>
          </ul>
        </AppCard>
      </div>
    </template>
  </div>
</template>
