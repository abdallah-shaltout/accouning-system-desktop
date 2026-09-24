<script setup lang="ts">
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SalesTrendChart from '@/modules/core/components/dashboard/SalesTrendChart.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ChartInsight from './ChartInsight.vue';
import HorizontalBarChart from './HorizontalBarChart.vue';
import { getSalesAnalytics } from '../services/analyticsService';

const { data, loading } = useAsync(() => getSalesAnalytics(30));
</script>

<template>
  <div class="space-y-4">
    <AppCard title="اتجاه المبيعات — آخر 30 يوماً" padding="sm">
      <SkeletonBlock v-if="loading" height="h-48" />
      <template v-else-if="data">
        <ChartInsight :text="data.trendInsight" />
        <SalesTrendChart :data="data.trend" />
      </template>
    </AppCard>

    <div class="grid gap-4 lg:grid-cols-2">
      <AppCard title="المبيعات حسب يوم الأسبوع" padding="sm">
        <SkeletonBlock v-if="loading" :lines="7" />
        <template v-else-if="data">
          <ChartInsight :text="data.weekdayInsight" />
          <HorizontalBarChart :data="data.byWeekday.map((w) => ({ label: w.day, value: w.avg, sub: `متوسط الفاتورة` }))" money />
        </template>
      </AppCard>

      <AppCard title="توزيع طرق الدفع" padding="sm">
        <SkeletonBlock v-if="loading" :lines="5" />
        <template v-else-if="data">
          <ChartInsight :text="data.paymentMix.length ? `الدفع النقدي وغيره موزع على ${data.paymentMix.length} طرق خلال هذه الفترة` : 'لا توجد بيانات كافية بعد'" />
          <HorizontalBarChart :data="data.paymentMix.map((m) => ({ label: m.method, value: m.total, sub: `${m.pct}%` }))" money />
        </template>
      </AppCard>
    </div>

    <div class="grid gap-4 sm:grid-cols-3">
      <AppCard padding="sm">
        <p class="text-xs text-text-secondary">متوسط قيمة الفاتورة</p>
        <p class="mt-1 text-stat font-semibold"><MoneyText :value="data?.avgInvoice" /></p>
      </AppCard>
      <AppCard padding="sm">
        <p class="text-xs text-text-secondary">متوسط الأصناف لكل فاتورة</p>
        <p class="mt-1 text-stat font-semibold"><span class="num">{{ formatNumber(data?.avgItemsPerInvoice, 1) }}</span></p>
      </AppCard>
      <AppCard padding="sm">
        <p class="text-xs text-text-secondary">نسبة المرتجعات</p>
        <p class="mt-1 text-stat font-semibold"><span class="num">{{ formatNumber(data?.returnsRatePct, 1) }}%</span></p>
      </AppCard>
    </div>
  </div>
</template>
