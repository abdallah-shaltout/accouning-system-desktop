<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import ScrollFade from '@/modules/core/components/ui/ScrollFade.vue';
import { useSettingsStore } from '../controllers/useSettingsStore';

const route = useRoute();
const auth = useAuthStore();
const settingsStore = useSettingsStore();

// v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §4): the three dimension tabs stay
// hidden until their feature switch is on — this is what keeps Settings looking exactly as it did
// before this phase for a single-branch, single-currency, no-cost-center business.
const tabs = computed(() =>
  [
    { to: { name: 'settings-general' } as const, label: 'عام', show: auth.can('settings') },
    { to: { name: 'settings-taxes' } as const, label: 'الضرائب', show: auth.can('settings') },
    { to: { name: 'settings-payment-methods' } as const, label: 'طرق الدفع', show: auth.can('settings') },
    { to: { name: 'settings-products' } as const, label: 'المنتجات', show: auth.can('inventory', 'write') },
    { to: { name: 'settings-branches' } as const, label: 'الفروع', show: auth.can('settings') && !!settingsStore.settings?.features?.branches },
    { to: { name: 'settings-cost-centers' } as const, label: 'مراكز التكلفة', show: auth.can('settings') && !!settingsStore.settings?.features?.costCenters },
    { to: { name: 'settings-currencies' } as const, label: 'العملات', show: auth.can('settings') && !!settingsStore.settings?.features?.currencies },
    { to: { name: 'settings-recommendations' } as const, label: 'التوصيات', show: auth.can('settings') },
    { to: { name: 'settings-roles' } as const, label: 'المستخدمون والأدوار', show: auth.can('users') },
    // 18.B4: business audit trail, admin-only (same gate as the roles tab).
    { to: { name: 'settings-audit-log' } as const, label: 'سجل التدقيق', show: auth.can('users') },
    { to: { name: 'settings-printing' } as const, label: 'الطباعة والأجهزة', show: auth.can('settings') },
    { to: { name: 'settings-appearance' } as const, label: 'المظهر', show: true },
    { to: { name: 'settings-keyboard-shortcuts' } as const, label: 'اختصارات لوحة المفاتيح', show: true },
    { to: { name: 'settings-backup' } as const, label: 'النسخ الاحتياطي', show: auth.can('settings') },
    // 18.B6: version info + support-bundle export, open to every signed-in user.
    { to: { name: 'settings-about' } as const, label: 'حول / الدعم', show: true },
  ].filter((t) => t.show),
);
</script>

<template>
  <ScrollFade class="mb-5 border-b border-border">
    <nav class="flex gap-1" aria-label="أقسام الإعدادات">
      <RouterLink
        v-for="t in tabs"
        :key="t.to.name"
        :to="t.to"
        class="-mb-px shrink-0 border-b-2 px-3 py-2 text-body transition-colors"
        :class="route.name === t.to.name ? 'border-primary font-medium text-text-primary' : 'border-transparent text-text-secondary hover:text-text-primary'"
      >
        {{ t.label }}
      </RouterLink>
    </nav>
  </ScrollFade>
</template>
