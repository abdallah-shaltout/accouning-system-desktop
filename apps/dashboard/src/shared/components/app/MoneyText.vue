<script setup lang="ts">
import { computed } from "vue";
import { formatMoney } from "@/shared/helpers/format";
import { DEFAULT_CURRENCY } from "@/shared/config/constants";

/**
 * Renders piasters as Arabic-formatted money, e.g. «299 ج.م» (CLAUDE.md "Money comes from the API in
 * piasters ... displayed only through MoneyText"). Numbers stay LTR inside RTL text via `.num`.
 */
interface Props {
  /** Amount in piasters. A string is accepted since JSON can't carry a real bigint. */
  amount: number | string;
  currency?: string;
}

const props = withDefaults(defineProps<Props>(), {
  currency: DEFAULT_CURRENCY,
});

const text = computed(() => formatMoney(props.amount, props.currency));
</script>

<template>
  <span class="num">{{ text }}</span>
</template>
