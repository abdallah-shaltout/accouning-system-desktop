<script setup lang="ts">
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import ChartInsight from './ChartInsight.vue';
import HorizontalBarChart from './HorizontalBarChart.vue';
import { getProductAnalytics } from '../services/analyticsService';

const { data, loading } = useAsync(() => getProductAnalytics(30, 8));
</script>

<template>
  <div class="space-y-4">
    <AppCard title="الأعلى ربحاً" padding="sm">
      <SkeletonBlock v-if="loading" :lines="8" />
      <template v-else-if="data">
        <ChartInsight :text="data.insight" />
        <HorizontalBarChart :data="data.top.map((p) => ({ label: p.name, value: p.grossProfit, sub: `هامش ${p.marginPct}%` }))" money />
      </template>
    </AppCard>

    <AppCard title="الأقل ربحاً (ضمن ما تم بيعه)" padding="sm">
      <SkeletonBlock v-if="loading" :lines="8" />
      <template v-else-if="data">
        <ChartInsight :text="data.bottom.length ? `${data.bottom[0].name} هو الأقل ربحاً بهامش ${data.bottom[0].marginPct}%` : 'لا توجد بيانات كافية بعد'" />
        <HorizontalBarChart :data="data.bottom.map((p) => ({ label: p.name, value: p.grossProfit, sub: `هامش ${p.marginPct}%` }))" money tone="danger" />
      </template>
    </AppCard>
  </div>
</template>
