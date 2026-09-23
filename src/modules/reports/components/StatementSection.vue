<script setup lang="ts">
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import type { StatementLine } from '../types';

/** One titled block of a financial statement: account lines + a subtotal. */
defineProps<{ title: string; lines: StatementLine[]; total: number; totalLabel?: string; emphasis?: boolean }>();
</script>

<template>
  <section class="overflow-hidden rounded-xl border border-border">
    <h3 class="border-b border-border bg-surface px-4 py-2 text-xs font-medium text-text-secondary">{{ title }}</h3>
    <ul class="divide-y divide-border text-body">
      <li v-for="l in lines" :key="l.accountId">
        <RouterLink :to="`/reports/ledger?account=${l.accountId}`" class="flex items-center justify-between px-4 py-2 hover:bg-surface-hover">
          <span class="flex items-center gap-2"><span class="num text-xs text-text-secondary">{{ l.code }}</span>{{ l.name }}</span>
          <MoneyText :value="l.amount" plain :class="l.amount < 0 && 'text-danger'" />
        </RouterLink>
      </li>
      <slot />
      <li v-if="!lines.length" class="px-4 py-2 text-xs text-text-secondary">لا توجد حركات</li>
    </ul>
    <div class="flex items-center justify-between border-t border-border bg-surface px-4 py-2.5 text-body font-semibold" :class="emphasis && 'text-ui'">
      <span>{{ totalLabel ?? `إجمالي ${title}` }}</span>
      <MoneyText :value="total" />
    </div>
  </section>
</template>
