<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §1 "Shift bar: shows the shift start time and cash sales so far"). */
import { computed } from 'vue';
import { Wallet } from '@lucide/vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatTime } from '@/modules/core/helpers/format';
import type { ShiftRow } from '../services/invoiceService';

const props = defineProps<{ shift: ShiftRow | null; now: Date }>();

const elapsed = computed(() => {
  if (!props.shift) return '';
  const ms = props.now.getTime() - new Date(props.shift.openedAt).getTime();
  const h = Math.floor(ms / 3600_000);
  const m = Math.floor((ms % 3600_000) / 60_000);
  return `${h}س ${m}د`;
});
</script>

<template>
  <div v-if="shift" class="flex items-center gap-2 rounded-full bg-primary/10 px-3 py-1 text-body text-primary" data-testid="shift-bar">
    <Wallet class="size-3.5" />
    <span>وردية {{ formatTime(shift.openedAt) }} · <span class="num">{{ elapsed }}</span></span>
    <span class="text-text-secondary">·</span>
    <span>نقدي <MoneyText :value="shift.cashSales" plain /></span>
  </div>
  <div v-else class="rounded-full bg-warning/10 px-3 py-1 text-body text-warning">لا توجد وردية مفتوحة</div>
</template>
