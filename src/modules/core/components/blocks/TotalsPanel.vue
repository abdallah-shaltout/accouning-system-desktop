<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 — the shared totals card for line-item documents (invoice, purchase, journal…):
 * rows of `{ label, amount, emphasis }` plus an optional tafqit (Arabic number-to-words) line. Pure
 * display — the amounts themselves must already be computed by the existing helpers (docs/v2/02
 * "tax-inclusive VAT, discount order: line → invoice → VAT"); this component never adds/subtracts.
 */
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { tafqit } from '@/modules/core/helpers/tafqit';

export interface TotalsRow {
  label: string;
  amount: number;
  /** Larger/bold row — typically the grand total. */
  emphasis?: boolean;
  /** Negative amounts (discounts, returns) render in the danger tone even when positive-valued. */
  negative?: boolean;
}

const props = withDefaults(
  defineProps<{
    rows: TotalsRow[];
    currency?: string;
    /** Shows the "فقط لا غير" amount-in-words line under the total (printed documents). */
    showTafqit?: boolean;
    tafqitAmount?: number;
  }>(),
  {},
);
</script>

<template>
  <div class="rounded-xl border border-border bg-surface p-4">
    <dl class="space-y-2">
      <div
        v-for="row in rows"
        :key="row.label"
        class="flex items-center justify-between gap-3"
        :class="row.emphasis && 'mt-2 border-t border-border pt-2'"
      >
        <dt class="text-body" :class="row.emphasis ? 'font-semibold' : 'text-text-secondary'">{{ row.label }}</dt>
        <dd :class="row.emphasis && 'text-lg font-semibold'">
          <MoneyText :value="row.negative ? -Math.abs(row.amount) : row.amount" :currency="currency" :signed="row.negative" />
        </dd>
      </div>
    </dl>
    <p v-if="showTafqit" class="mt-3 border-t border-border pt-2 text-xs text-text-secondary">
      {{ tafqit(tafqitAmount ?? rows.find((r) => r.emphasis)?.amount ?? 0, { currency }) }}
    </p>
  </div>
</template>
