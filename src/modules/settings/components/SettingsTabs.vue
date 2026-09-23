<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useSettingsStore } from '../controllers/useSettingsStore';

const route = useRoute();
const auth = useAuthStore();
const settingsStore = useSettingsStore();

// v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §4): the three dimension tabs stay
// hidden until their feature switch is on — this is what keeps Settings looking exactly as it did
// before this phase for a single-branch, single-currency, no-cost-center business.
const tabs = computed(() =>
  [
    { to: '/settings/general', label: 'عام', show: auth.can('settings') },
    { to: '/settings/taxes', label: 'الضرائب', show: auth.can('settings') },
    { to: '/settings/payment-methods', label: 'طرق الدفع', show: auth.can('settings') },
    { to: '/settings/products', label: 'المنتجات', show: auth.can('inventory', 'write') },
    { to: '/settings/branches', label: 'الفروع', show: auth.can('settings') && !!settingsStore.settings?.features?.branches },
    { to: '/settings/cost-centers', label: 'مراكز التكلفة', show: auth.can('settings') && !!settingsStore.settings?.features?.costCenters },
    { to: '/settings/currencies', label: 'العملات', show: auth.can('settings') && !!settingsStore.settings?.features?.currencies },
    { to: '/settings/roles', label: 'المستخدمون والأدوار', show: auth.can('users') },
    { to: '/settings/printing', label: 'الطباعة والأجهزة', show: auth.can('settings') },
    { to: '/settings/appearance', label: 'المظهر', show: true },
    { to: '/settings/backup', label: 'النسخ الاحتياطي', show: auth.can('settings') },
  ].filter((t) => t.show),
);
</script>

<template>
  <nav class="mb-5 flex gap-1 border-b border-border" aria-label="أقسام الإعدادات">
    <RouterLink
      v-for="t in tabs"
      :key="t.to"
      :to="t.to"
      class="-mb-px border-b-2 px-3 py-2 text-body transition-colors"
      :class="route.path === t.to ? 'border-primary font-medium text-text-primary' : 'border-transparent text-text-secondary hover:text-text-primary'"
    >
      {{ t.label }}
    </RouterLink>
  </nav>
</template>
