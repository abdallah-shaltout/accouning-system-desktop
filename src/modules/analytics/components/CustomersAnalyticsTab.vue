<script setup lang="ts">
import { computed } from 'vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ChartInsight from './ChartInsight.vue';
import HorizontalBarChart from './HorizontalBarChart.vue';
import { getCustomerAnalytics } from '../services/analyticsService';

const { data, loading } = useAsync(() => getCustomerAnalytics(30, 8));

const segmentData = computed(() =>
  data.value ? [{ label: 'جدد', value: data.value.newCount }, { label: 'عائدون', value: data.value.returningCount }] : [],
);
</script>

<template>
  <div class="space-y-4">
    <div class="grid gap-4 lg:grid-cols-2">
      <AppCard title="عملاء جدد مقابل عائدين" padding="sm">
        <SkeletonBlock v-if="loading" :lines="4" />
        <template v-else-if="data">
          <ChartInsight :text="data.segmentInsight" />
          <HorizontalBarChart :data="segmentData" />
          <p class="mt-3 text-xs text-text-secondary">
            <span class="num">{{ formatNumber(data.newCount) }}</span> جديد ·
            <span class="num">{{ formatNumber(data.returningCount) }}</span> عائد خلال آخر 30 يوماً
          </p>
        </template>
      </AppCard>

      <AppCard title="تركّز الإيرادات" padding="sm">
        <SkeletonBlock v-if="loading" :lines="4" />
        <template v-else-if="data">
          <ChartInsight :text="data.concentrationInsight" />
          <div class="flex items-center justify-center py-6">
            <p class="text-center">
              <span class="num block text-3xl font-semibold">{{ data.top10SharePct }}%</span>
              <span class="mt-1 block text-xs text-text-secondary">من الإيرادات من أفضل 10 عملاء</span>
            </p>
          </div>
        </template>
      </AppCard>
    </div>

    <AppCard title="أفضل العملاء" padding="sm">
      <SkeletonBlock v-if="loading" :lines="8" />
      <HorizontalBarChart v-else-if="data" :data="data.topCustomers.map((c) => ({ label: c.name, value: c.total }))" money />
    </AppCard>
  </div>
</template>
