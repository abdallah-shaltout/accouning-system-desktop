<script setup lang="ts">
import { computed } from 'vue';
import { formatMoney } from '../../helpers/format';
import { profileByCurrency } from '../../helpers/countryProfiles';
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
    /**
     * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §2): show this currency's symbol
     * instead of the store's base currency — for an FC document/line (`invoice.currency`,
     * `journalLine.currency`). Omitted = base currency (every pre-phase-9 caller, unchanged).
     */
    currency?: string;
  }>(),
  {},
);

const settings = useSettingsStore();
const displayCurrency = computed(() => props.currency ?? settings.currency);
// v2 doc 18.D: a symbol per active currency — `RiyalIcon` (the drawn glyph) only for SAR, the
// profile's Arabic symbol ("ج.م" for EGP, "ر.س" for any other/unknown currency) otherwise.
const currencySymbol = computed(() => profileByCurrency(displayCurrency.value).currency.symbolAr);
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
        <RiyalIcon v-if="displayCurrency === 'SAR'" class="opacity-70" />
        <span v-else class="text-[0.85em] opacity-70">{{ currencySymbol }}</span>
      </template>
    </template>
  </span>
</template>
