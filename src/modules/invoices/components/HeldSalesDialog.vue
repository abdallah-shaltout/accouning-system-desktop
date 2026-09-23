<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §1 "Held sales (F6)"). Lists held carts for this terminal; resume or discard. */
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { PauseCircle, Trash } from '@lucide/vue';
import type { HeldSale } from '../types';

defineProps<{ held: HeldSale[] }>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ resume: [id: string]; discard: [id: string] }>();
</script>

<template>
  <AppModal v-model:open="open" title="المبيعات المعلّقة" size="md">
    <EmptyState v-if="!held.length" :icon="PauseCircle" title="لا توجد مبيعات معلّقة" description="اضغط F6 لتعليق السلة الحالية" />
    <ul v-else class="divide-y divide-border">
      <li v-for="h in held" :key="h.id" class="flex items-center justify-between gap-3 py-2.5">
        <div class="min-w-0">
          <p class="truncate text-body font-medium">{{ h.label || `${formatNumber(h.lines.length)} صنف` }}</p>
          <p class="text-xs text-text-secondary">{{ formatDateTime(h.heldAt) }} · <MoneyText :value="h.lines.reduce((a, l) => a + l.qty * l.price, 0)" plain /></p>
        </div>
        <div class="flex shrink-0 gap-1.5">
          <AppButton size="sm" variant="primary" @click="emit('resume', h.id)">استئناف</AppButton>
          <AppButton size="sm" variant="ghost" :icon="Trash" aria-label="حذف" @click="emit('discard', h.id)" />
        </div>
      </li>
    </ul>
  </AppModal>
</template>
