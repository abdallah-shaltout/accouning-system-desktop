<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §1 "comparison mode: this period vs last, or vs same period last year"). */
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import type { ComparisonMode } from '../types';

const model = defineModel<ComparisonMode>({ default: 'none' });
</script>

<template>
  <SegmentedControl
    v-model="model"
    :options="[
      { value: 'none', label: 'بدون مقارنة' },
      { value: 'previousPeriod', label: 'الفترة السابقة' },
      { value: 'sameLastYear', label: 'نفس الفترة العام الماضي' },
    ]"
  />
</template>
