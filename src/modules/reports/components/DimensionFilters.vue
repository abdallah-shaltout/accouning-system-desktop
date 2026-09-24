<script setup lang="ts">
/**
 * v2 phase 12 (docs/v2/13-reports.md §1, docs/v2/10-branches-currencies-cost-centers.md §4): the
 * branch/cost-center/currency selects on `ReportShell`'s filter bar — each rendered only when its
 * feature switch is on (the visibility booleans are computed by `useReportFilters`).
 */
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';

defineProps<{
  showBranch: boolean;
  showCostCenter: boolean;
  showCurrency: boolean;
  branches: { id: string; label: string }[];
  costCenters: { id: string; label: string }[];
  currencies: { code: string; label: string }[];
}>();

const branchId = defineModel<string>('branchId', { default: '' });
const costCenterId = defineModel<string>('costCenterId', { default: '' });
const currency = defineModel<string>('currency', { default: '' });
</script>

<template>
  <AppSelect v-if="showBranch" v-model="branchId" class="w-40" placeholder="كل الفروع" :options="branches.map((b) => ({ value: b.id, label: b.label }))" />
  <AppSelect v-if="showCostCenter" v-model="costCenterId" class="w-44" placeholder="كل مراكز التكلفة" :options="costCenters.map((c) => ({ value: c.id, label: c.label }))" />
  <AppSelect v-if="showCurrency" v-model="currency" class="w-36" placeholder="العملة الأساسية" :options="currencies.map((c) => ({ value: c.code, label: c.label }))" />
</template>
