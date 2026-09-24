<script setup lang="ts">
import { ref } from 'vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SalesAnalyticsTab from '../components/SalesAnalyticsTab.vue';
import ProductsAnalyticsTab from '../components/ProductsAnalyticsTab.vue';
import CustomersAnalyticsTab from '../components/CustomersAnalyticsTab.vue';

/**
 * `/analytics` (التحليلات) — docs/v2/11-journal-dashboard-insights.md Part C: "Depth lives here."
 * A few solid tabs rather than forcing exhaustive coverage of the doc's full table (see the
 * services file for what's built vs skipped this phase).
 */
type Tab = 'sales' | 'products' | 'customers';
const tab = ref<Tab>('sales');
const TABS: { value: Tab; label: string }[] = [
  { value: 'sales', label: 'المبيعات' },
  { value: 'products', label: 'المنتجات' },
  { value: 'customers', label: 'العملاء' },
];
</script>

<template>
  <div>
    <PageHeader title="التحليلات" subtitle="رؤى تلقائية بجملة واحدة أعلى كل رسم بياني" />

    <nav class="mb-5 flex gap-1 border-b border-border" aria-label="أقسام التحليلات">
      <button
        v-for="t in TABS"
        :key="t.value"
        type="button"
        class="-mb-px border-b-2 px-3 py-2 text-body transition-colors"
        :class="tab === t.value ? 'border-primary font-medium text-text-primary' : 'border-transparent text-text-secondary hover:text-text-primary'"
        @click="tab = t.value"
      >
        {{ t.label }}
      </button>
    </nav>

    <SalesAnalyticsTab v-if="tab === 'sales'" />
    <ProductsAnalyticsTab v-else-if="tab === 'products'" />
    <CustomersAnalyticsTab v-else-if="tab === 'customers'" />
  </div>
</template>
