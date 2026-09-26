<script setup lang="ts">
/**
 * v2 phase 5 (docs/v2/05-onboarding.md §2 "Afterwards"): a setup-checklist card for skipped
 * onboarding steps (opening balances, users, template, first backup…), shown on the owner/manager
 * dashboard (`DashboardPage.vue`, below `NeedsAttentionPanel`) once setup is incomplete.
 *
 * Stays its own simple "skipped steps" checklist rather than becoming insight-engine rules
 * (docs/v2/11 Part D): onboarding progress is a one-time, per-company completion state (11 fixed
 * wizard steps), not a recurring condition to re-evaluate against live ledger/catalog data the way
 * "20 items below reorder point" or "VAT deadline in 5 days" are — there's nothing for a rule to
 * recompute here beyond "is this step still marked done", so a plain progress check is the right
 * shape, not a gap.
 */
import { onMounted, ref } from 'vue';
import { Check } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import DirIcon from '@/modules/core/components/ui/DirIcon.vue';
import { dirIcon } from '@/modules/core/helpers/dirIcon';
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
        <RouterLink :to="{ name: 'setup-wizard' }" class="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-xs hover:bg-surface-hover">
          <span>{{ p.label }}</span>
          <DirIcon :icon="dirIcon.open" class="size-3.5 text-text-secondary" />
        </RouterLink>
      </li>
    </ul>
  </AppCard>
  <AppCard v-else-if="loaded" title="الإعداد">
    <p class="flex items-center gap-2 text-xs text-success"><Check class="size-4" /> اكتمل إعداد الشركة</p>
  </AppCard>
</template>
