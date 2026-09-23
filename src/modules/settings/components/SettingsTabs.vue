<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';

const route = useRoute();
const auth = useAuthStore();

const tabs = computed(() =>
  [
    { to: '/settings/general', label: 'عام', show: auth.can('settings') },
    { to: '/settings/taxes', label: 'الضرائب', show: auth.can('settings') },
    { to: '/settings/payment-methods', label: 'طرق الدفع', show: auth.can('settings') },
    { to: '/settings/products', label: 'المنتجات', show: auth.can('inventory', 'write') },
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
