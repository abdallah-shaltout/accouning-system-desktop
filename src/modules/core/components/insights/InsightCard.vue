<script setup lang="ts">
import { Check, MoreHorizontal, X } from '@lucide/vue';
import { ref } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import type { Insight } from '@/modules/core/services/insightTypes';

/**
 * One "يحتاج انتباهك" card (docs/v2/11 Part B.2): severity icon, one sentence, a metric and one
 * primary action button, plus a small overflow menu for dismiss/snooze (D1).
 */
const props = defineProps<{ insight: Insight }>();
const emit = defineEmits<{ dismiss: []; snooze: [] }>();

const menuOpen = ref(false);

const SEVERITY_CLASS: Record<Insight['severity'], string> = {
  critical: 'bg-danger/10 text-danger',
  warning: 'bg-warning/10 text-warning',
  info: 'bg-primary/10 text-primary',
  positive: 'bg-success/10 text-success',
};

function onDismiss() {
  menuOpen.value = false;
  emit('dismiss');
}
function onSnooze() {
  menuOpen.value = false;
  emit('snooze');
}
</script>

<template>
  <div class="group relative flex items-start gap-3 rounded-xl border border-border bg-surface p-3.5">
    <span class="flex size-8 shrink-0 items-center justify-center rounded-lg" :class="SEVERITY_CLASS[insight.severity]">
      <component :is="insight.icon" class="size-4" :stroke-width="1.75" />
    </span>
    <div class="min-w-0 flex-1">
      <p class="text-body leading-5">{{ insight.message }}</p>
      <div class="mt-2 flex items-center gap-2">
        <AppButton v-if="insight.severity !== 'positive'" size="sm" variant="secondary" :to="insight.actionTo">{{ insight.actionLabel }}</AppButton>
        <span v-else class="text-xs text-text-secondary">{{ insight.metric }}</span>
      </div>
    </div>
    <div v-if="insight.severity !== 'positive'" class="relative shrink-0">
      <button
        type="button"
        class="rounded-md p-1 text-text-secondary opacity-0 transition-opacity hover:bg-surface-hover hover:text-text-primary group-hover:opacity-100"
        :class="menuOpen && 'opacity-100'"
        aria-label="خيارات"
        @click="menuOpen = !menuOpen"
      >
        <MoreHorizontal class="size-4" />
      </button>
      <div v-if="menuOpen" class="absolute start-0 top-7 z-10 w-32 rounded-lg border border-border bg-surface py-1 shadow-lg">
        <button type="button" class="flex w-full items-center gap-2 px-3 py-1.5 text-start text-xs hover:bg-surface-hover" @click="onSnooze">
          تأجيل أسبوع
        </button>
        <button type="button" class="flex w-full items-center gap-2 px-3 py-1.5 text-start text-xs text-danger hover:bg-danger/10" @click="onDismiss">
          <X class="size-3.5" /> إخفاء
        </button>
      </div>
    </div>
    <Check v-else class="size-4 shrink-0 text-success" :stroke-width="1.75" />
  </div>
</template>
