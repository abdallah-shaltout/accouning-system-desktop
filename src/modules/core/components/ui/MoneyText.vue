<script setup lang="ts">
import { computed } from 'vue';
import { formatMoney } from '../../helpers/format';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import RiyalIcon from './RiyalIcon.vue';

const props = withDefaults(
  defineProps<{
    value: number | undefined | null;
    /** Color negatives red / positives green (balances, P&L). */
    signed?: boolean;
    /** Hide the currency symbol (dense tables where the header already says the currency). */
    plain?: boolean;
    /** Render zero as a dash. */
    dashZero?: boolean;
  }>(),
  {},
);

const settings = useSettingsStore();
const isZero = computed(() => Math.abs(props.value ?? 0) < 0.005);
const tone = computed(() => {
  if (!props.signed || isZero.value) return '';
  return (props.value ?? 0) < 0 ? 'text-danger' : 'text-success';
});
</script>

<template>
  <span class="inline-flex items-baseline gap-1 whitespace-nowrap" :class="tone">
    <template v-if="dashZero && isZero"><span class="text-text-secondary">—</span></template>
    <template v-else>
      <span class="num">{{ formatMoney(value) }}</span>
      <template v-if="!plain">
        <RiyalIcon v-if="settings.currency === 'SAR'" class="opacity-70" />
        <span v-else class="text-[0.85em] opacity-70">{{ settings.currency }}</span>
      </template>
    </template>
  </span>
</template>
