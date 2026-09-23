<script setup lang="ts">
import { computed } from 'vue';
import { daysAgoKey, startOfMonthKey, todayKey } from '../../helpers/format';

const from = defineModel<string>('from', { default: '' });
const to = defineModel<string>('to', { default: '' });
const props = defineProps<{ fiscalStart?: string }>();

const presets = computed(() => {
  const now = new Date();
  const list = [
    { label: 'اليوم', from: todayKey(), to: todayKey() },
    { label: '7 أيام', from: daysAgoKey(6), to: todayKey() },
    { label: 'هذا الشهر', from: startOfMonthKey(now), to: todayKey() },
    { label: '30 يوماً', from: daysAgoKey(29), to: todayKey() },
  ];
  if (props.fiscalStart) list.push({ label: 'السنة المالية', from: props.fiscalStart, to: todayKey() });
  list.push({ label: 'الكل', from: '', to: '' });
  return list;
});

function apply(p: { from: string; to: string }) {
  from.value = p.from;
  to.value = p.to;
}
</script>

<template>
  <div class="flex flex-wrap items-center gap-2">
    <div class="flex items-center gap-1.5">
      <input v-model="from" type="date" dir="ltr" class="control h-[30px] w-[132px] text-xs" aria-label="من تاريخ" />
      <span class="text-xs text-text-secondary">إلى</span>
      <input v-model="to" type="date" dir="ltr" class="control h-[30px] w-[132px] text-xs" aria-label="إلى تاريخ" />
    </div>
    <div class="flex flex-wrap gap-1">
      <button
        v-for="p in presets"
        :key="p.label"
        type="button"
        class="h-[26px] rounded-full border px-2.5 text-xs transition-colors"
        :class="
          from === p.from && to === p.to
            ? 'border-primary/40 bg-primary/10 text-primary'
            : 'border-border text-text-secondary hover:bg-surface-hover hover:text-text-primary'
        "
        @click="apply(p)"
      >
        {{ p.label }}
      </button>
    </div>
  </div>
</template>
