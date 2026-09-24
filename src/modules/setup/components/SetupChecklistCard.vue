<script setup lang="ts">
/**
 * v2 phase 5 (docs/v2/05-onboarding.md §2 "Afterwards"): a setup-checklist card for skipped
 * onboarding steps (opening balances, users, template, first backup…), meant for the dashboard.
 *
 * Ownership note: Phase 10 (docs/v2/11-journal-dashboard-insights.md) owns `modules/core`'s
 * dashboard pages and the insight engine. This component is deliberately standalone — NOT wired
 * into `DashboardPage.vue` by this phase, to avoid touching a file Phase 10 is editing
 * concurrently. Phase 10 (or a later pass) drops `<SetupChecklistCard />` into the dashboard grid.
 * TODO(phase 10): replace this simple "skipped steps" render with a real insight-engine card
 * (dismissible, ranked by impact) once `modules/core/services/insightTypes.ts`'s engine exists.
 */
import { onMounted, ref } from 'vue';
import { Check, ChevronLeft } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import { getOnboardingProgress } from '../services/setupService';
import { WIZARD_STEPS } from '../types';

const pending = ref<{ key: string; label: string }[]>([]);
const loaded = ref(false);

onMounted(async () => {
  const progress = await getOnboardingProgress();
  const done = new Set(progress.done);
  pending.value = WIZARD_STEPS.filter((s) => s.key !== 'ready' && !done.has(s.key) && (progress.skipped.includes(s.key) || !done.has(s.key))).map((s) => ({
    key: s.key,
    label: s.label,
  }));
  loaded.value = true;
});
</script>

<template>
  <AppCard v-if="loaded && pending.length" title="إكمال الإعداد">
    <ul class="space-y-1.5">
      <li v-for="p in pending" :key="p.key">
        <RouterLink to="/setup" class="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-xs hover:bg-surface-hover">
          <span>{{ p.label }}</span>
          <ChevronLeft class="size-3.5 text-text-secondary" />
        </RouterLink>
      </li>
    </ul>
  </AppCard>
  <AppCard v-else-if="loaded" title="الإعداد">
    <p class="flex items-center gap-2 text-xs text-success"><Check class="size-4" /> اكتمل إعداد الشركة</p>
  </AppCard>
</template>
