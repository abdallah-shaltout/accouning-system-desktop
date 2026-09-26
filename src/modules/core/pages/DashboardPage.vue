<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Banknote, Plus, ReceiptText, ShoppingCart, TrendingUp } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import AppButton from '../components/ui/AppButton.vue';
import AppCard from '../components/ui/AppCard.vue';
import EmptyState from '../components/ui/EmptyState.vue';
import ErrorState from '../components/ui/ErrorState.vue';
import MoneyText from '../components/ui/MoneyText.vue';
import RiyalIcon from '../components/ui/RiyalIcon.vue';
import SkeletonBlock from '../components/ui/SkeletonBlock.vue';
import KpiCard from '../components/dashboard/KpiCard.vue';
import Sparkline from '../components/dashboard/Sparkline.vue';
import SalesTrendChart from '../components/dashboard/SalesTrendChart.vue';
import StorekeeperHome from '../components/dashboard/StorekeeperHome.vue';
import CashierHome from '../components/dashboard/CashierHome.vue';
import AccountantHome from '../components/dashboard/AccountantHome.vue';
import NeedsAttentionPanel from '../components/insights/NeedsAttentionPanel.vue';
import SetupChecklistCard from '@/modules/setup/components/SetupChecklistCard.vue';
import { useAsync } from '../controllers/useAsync';
import { formatDateLong, formatMoney, formatNumber } from '../helpers/format';
import { getHomeKpis, getTopCustomers, getTopProducts, type HomePeriod } from '../services/dashboardService';

/**
 * v2 phase 10 (docs/v2/11-journal-dashboard-insights.md Part B "simpler home"): answers three
 * questions in order — what needs me → how are we doing → what's the trend. Everything else
 * (activity feed, recent invoices, the old low-stock table) moved to the bell/insights drawer,
 * the invoices page, and the insight engine respectively.
 *
 * Role homes (same components, different content — doc's table): cashier's home is the POS itself
 * plus a shift panel (`CashierHome.vue`); storekeeper gets the full insight-driven home
 * (`StorekeeperHome.vue`, completing the stub Phase 6 left for this phase); accountant gets a
 * posting/approval-focused home (`AccountantHome.vue`); owner/manager/admin get the full home below.
 */
const auth = useAuthStore();

const isStorekeeper = computed(() => auth.role === 'storekeeper');
const isCashier = computed(() => auth.role === 'cashier');
const isAccountant = computed(() => auth.role === 'accountant');
const isFullHome = computed(() => !isStorekeeper.value && !isCashier.value && !isAccountant.value);

const PERIODS: { value: HomePeriod; label: string }[] = [
  { value: 'today', label: 'اليوم' },
  { value: 'week', label: 'هذا الأسبوع' },
  { value: 'month', label: 'هذا الشهر' },
];
const period = ref<HomePeriod>('today');

const kpis = useAsync(() => getHomeKpis(period.value));
const topProducts = useAsync(() => getTopProducts(period.value));
const topCustomers = useAsync(() => getTopCustomers(period.value));

watch(period, () => {
  kpis.reload();
  topProducts.reload();
  topCustomers.reload();
});

const canSeeCash = computed(() => auth.can('accounting'));

const greeting = computed(() => {
  const h = new Date().getHours();
  return h < 12 ? 'صباح الخير' : 'مساء الخير';
});

function retryAll() {
  kpis.reload();
  topProducts.reload();
  topCustomers.reload();
}
</script>

<template>
  <StorekeeperHome v-if="isStorekeeper" />
  <CashierHome v-else-if="isCashier" />
  <AccountantHome v-else-if="isAccountant" />
  <div v-else-if="isFullHome">
    <div class="mb-5 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 class="text-lg font-semibold tracking-tight">{{ greeting }}، {{ auth.user?.name.split(' ')[0] }}</h1>
        <p class="mt-0.5 text-body text-text-secondary">{{ formatDateLong(new Date().toISOString()) }}</p>
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <div class="flex rounded-lg border border-border p-0.5">
          <button
            v-for="p in PERIODS"
            :key="p.value"
            type="button"
            class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors"
            :class="period === p.value ? 'bg-primary text-on-primary' : 'text-text-secondary hover:text-text-primary'"
            @click="period = p.value"
          >
            {{ p.label }}
          </button>
        </div>
        <div class="flex gap-2">
          <AppButton v-if="auth.can('purchases', 'write')" :icon="Plus" :to="{ name: 'purchase-new' }">أمر شراء</AppButton>
          <AppButton v-if="auth.can('pos', 'write')" variant="primary" :icon="ShoppingCart" :to="{ name: 'pos' }">بيع جديد</AppButton>
        </div>
      </div>
    </div>

    <ErrorState v-if="kpis.error.value" :message="kpis.error.value" @retry="retryAll" />

    <template v-else>
      <!-- "يحتاج انتباهك" (docs/v2/11 Part B.2) -->
      <div class="mb-4">
        <NeedsAttentionPanel />
      </div>

      <!-- Setup checklist (docs/v2/05-onboarding.md §2 "Afterwards") — hides itself once complete. -->
      <div class="mb-4">
        <SetupChecklistCard />
      </div>

      <!-- 4 KPIs with period-over-period comparison + sparkline (Part B.3) -->
      <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <KpiCard label="صافي المبيعات" :icon="TrendingUp" :loading="kpis.loading.value" :change-pct="kpis.data.value?.netSales.changePct" :to="auth.can('sales') ? { name: 'invoices' } : undefined">
          <span dir="ltr">{{ formatMoney(kpis.data.value?.netSales.value) }}</span> <RiyalIcon class="text-[0.7em] text-text-secondary" />
          <template #hint>شامل الضريبة وبعد المرتجعات</template>
          <template #spark><Sparkline :data="kpis.data.value?.netSales.sparkline ?? []" /></template>
        </KpiCard>

        <KpiCard label="مجمل الربح" :icon="Banknote" :loading="kpis.loading.value" :change-pct="kpis.data.value?.grossProfit.changePct" :to="canSeeCash ? { name: 'report-profit-loss' } : undefined">
          <span dir="ltr">{{ formatMoney(kpis.data.value?.grossProfit.value) }}</span> <RiyalIcon class="text-[0.7em] text-text-secondary" />
          <template #hint>هامش {{ formatNumber(kpis.data.value?.grossProfit.marginPct, 1) }}%</template>
          <template #spark><Sparkline :data="kpis.data.value?.grossProfit.sparkline ?? []" tone="success" /></template>
        </KpiCard>

        <KpiCard v-if="canSeeCash" label="السيولة" :icon="Banknote" :loading="kpis.loading.value" :change-pct="kpis.data.value?.cash.changePct" :to="{ name: 'report-ledger', query: { account: 'acc-1110' } }">
          <span dir="ltr">{{ formatMoney(kpis.data.value?.cash.value) }}</span> <RiyalIcon class="text-[0.7em] text-text-secondary" />
          <template #hint>الصندوق والبنك، شامل التسوية</template>
          <template #spark><Sparkline :data="kpis.data.value?.cash.sparkline ?? []" /></template>
        </KpiCard>

        <KpiCard
          label="مستحق من العملاء"
          :icon="ReceiptText"
          :loading="kpis.loading.value"
          :change-pct="kpis.data.value?.receivables.changePct"
          :tone="(kpis.data.value?.receivables.overdue ?? 0) > 0 ? 'warning' : undefined"
          :to="auth.can('sales') ? { name: 'invoices', query: { payment: 'open' } } : undefined"
        >
          <span dir="ltr">{{ formatMoney(kpis.data.value?.receivables.value) }}</span> <RiyalIcon class="text-[0.7em] text-text-secondary" />
          <template #hint>
            متأخر: <span class="num text-warning">{{ formatMoney(kpis.data.value?.receivables.overdue) }}</span>
          </template>
          <template #spark><Sparkline :data="kpis.data.value?.receivables.sparkline ?? []" tone="danger" /></template>
        </KpiCard>
      </div>

      <!-- One comparison chart (Part B.4) -->
      <div class="mt-4">
        <AppCard title="صافي المبيعات — مقارنة بالفترة السابقة" padding="sm">
          <SkeletonBlock v-if="kpis.loading.value" height="h-48" />
          <SalesTrendChart v-else-if="kpis.data.value" :data="kpis.data.value.salesTrend" />
        </AppCard>
      </div>

      <!-- Top products / customers (Part B.5) -->
      <div class="mt-4 grid gap-4 lg:grid-cols-2">
        <AppCard title="أفضل 5 منتجات (حسب الربح)" padding="none">
          <div v-if="topProducts.loading.value" class="p-4"><SkeletonBlock :lines="5" /></div>
          <EmptyState v-else-if="!topProducts.data.value?.length" title="لا توجد مبيعات في هذه الفترة" compact />
          <ul v-else class="divide-y divide-border">
            <li v-for="p in topProducts.data.value" :key="p.id">
              <RouterLink :to="{ name: 'product', params: { id: p.id } }" class="flex items-center justify-between gap-3 px-4 py-2.5 hover:bg-surface-hover">
                <span class="min-w-0">
                  <span class="block truncate text-body">{{ p.name }}</span>
                  <span class="num block text-tiny text-text-secondary">{{ p.sku }} · {{ formatNumber(p.qty) }} وحدة</span>
                </span>
                <MoneyText :value="p.grossProfit" class="shrink-0" />
              </RouterLink>
            </li>
          </ul>
        </AppCard>

        <AppCard title="أفضل 5 عملاء" padding="none">
          <div v-if="topCustomers.loading.value" class="p-4"><SkeletonBlock :lines="5" /></div>
          <EmptyState v-else-if="!topCustomers.data.value?.length" title="لا توجد مبيعات لعملاء في هذه الفترة" compact />
          <ul v-else class="divide-y divide-border">
            <li v-for="c in topCustomers.data.value" :key="c.id">
              <RouterLink :to="{ name: 'customer', params: { id: c.id } }" class="flex items-center justify-between gap-3 px-4 py-2.5 hover:bg-surface-hover">
                <span class="truncate text-body">{{ c.name }}</span>
                <MoneyText :value="c.total" class="shrink-0" />
              </RouterLink>
            </li>
          </ul>
        </AppCard>
      </div>
    </template>
  </div>
</template>
