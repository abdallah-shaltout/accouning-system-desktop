<script setup lang="ts">
import { computed } from 'vue';
import MoneyText from './ui/MoneyText.vue';

/** Compact debit/credit table used to preview (or show) the double-entry behind a document. */
const props = defineProps<{
  lines: { accountCode: string; accountName: string; debit: number; credit: number }[];
  title?: string;
}>();

const totals = computed(() => ({
  debit: props.lines.reduce((a, l) => a + l.debit, 0),
  credit: props.lines.reduce((a, l) => a + l.credit, 0),
}));
</script>

<template>
  <div class="overflow-hidden rounded-lg border border-border">
    <p v-if="title" class="border-b border-border bg-surface px-3 py-2 text-xs font-medium text-text-secondary">{{ title }}</p>
    <table class="w-full text-xs">
      <thead class="text-text-secondary">
        <tr class="border-b border-border">
          <th class="px-3 py-1.5 text-start font-medium">الحساب</th>
          <th class="px-3 py-1.5 text-start font-medium">مدين</th>
          <th class="px-3 py-1.5 text-start font-medium">دائن</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(l, i) in lines" :key="i" class="border-b border-border last:border-0">
          <td class="px-3 py-1.5" :class="l.credit > 0 && 'ps-7'">
            <span class="inline-flex items-center gap-1.5"><span class="num text-text-secondary">{{ l.accountCode }}</span>{{ l.accountName }}</span>
          </td>
          <td class="px-3 py-1.5"><MoneyText v-if="l.debit" :value="l.debit" plain /></td>
          <td class="px-3 py-1.5"><MoneyText v-if="l.credit" :value="l.credit" plain /></td>
        </tr>
      </tbody>
      <tfoot class="border-t border-border bg-surface font-medium">
        <tr>
          <td class="px-3 py-1.5">الإجمالي</td>
          <td class="px-3 py-1.5"><MoneyText :value="totals.debit" plain /></td>
          <td class="px-3 py-1.5"><MoneyText :value="totals.credit" plain /></td>
        </tr>
      </tfoot>
    </table>
  </div>
</template>
