<script setup lang="ts">
/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §5 "Close: the cashier counts the cash (by denomination),
 * and the app shows expected vs counted. Any variance is posted to cash over (4330) or cash short
 * (6320). Z-report is printed. The cashier chooses to hand over... or drop cash to the safe/bank").
 */
import { computed, ref, watch } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { num0, round2 } from '@/modules/core/helpers/numbers';
import type { DenominationCount } from '../types';
import type { ShiftRow } from '../services/invoiceService';

const DENOMS = [500, 200, 100, 50, 20, 10, 5, 1];

const props = defineProps<{ shift: ShiftRow | null; submitting?: boolean }>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ confirm: [countedCash: number, denominations: DenominationCount[] | undefined, handoverMode: 'HANDOVER' | 'DROP', note?: string] }>();

const useDenoms = ref(false);
const counts = ref<Record<number, number>>({});
const flatCounted = ref<number | undefined>();
const handoverMode = ref<'HANDOVER' | 'DROP'>('HANDOVER');
const note = ref('');

watch(open, (o) => {
  if (o) {
    counts.value = Object.fromEntries(DENOMS.map((d) => [d, 0]));
    flatCounted.value = props.shift ? round2(props.shift.expectedCash) : undefined;
    useDenoms.value = false;
    handoverMode.value = 'HANDOVER';
    note.value = '';
  }
});

const denomTotal = computed(() => DENOMS.reduce((a, d) => a + d * num0(counts.value[d]), 0));
const counted = computed(() => (useDenoms.value ? denomTotal.value : num0(flatCounted.value)));
const variance = computed(() => round2(counted.value - (props.shift?.expectedCash ?? 0)));

function confirm() {
  const denoms: DenominationCount[] | undefined = useDenoms.value ? DENOMS.map((value) => ({ value, count: num0(counts.value[value]) })) : undefined;
  emit('confirm', counted.value, denoms, handoverMode.value, note.value.trim() || undefined);
}
</script>

<template>
  <AppModal v-model:open="open" title="إغلاق الوردية" size="md" :persistent="submitting">
    <div v-if="shift" class="space-y-4">
      <dl class="grid grid-cols-2 gap-3 rounded-lg border border-border bg-surface p-3 text-body">
        <div><dt class="text-xs text-text-secondary">الرصيد الافتتاحي</dt><dd><MoneyText :value="shift.openingFloat" plain /></dd></div>
        <div><dt class="text-xs text-text-secondary">النقد المتوقع</dt><dd class="font-medium"><MoneyText :value="shift.expectedCash" plain /></dd></div>
        <div v-for="s in shift.salesByMethod" :key="s.label"><dt class="text-xs text-text-secondary">{{ s.label }}</dt><dd><MoneyText :value="s.amount" plain /></dd></div>
      </dl>

      <label class="flex items-center gap-2 text-body">
        <input v-model="useDenoms" type="checkbox" class="size-4" />
        عدّ النقد حسب الفئات
      </label>
      <div v-if="!useDenoms">
        <label class="field-label" for="counted-cash">النقد المعدود</label>
        <input id="counted-cash" v-model.number="flatCounted" type="number" min="0" step="0.01" class="control h-11 text-lg" autofocus />
      </div>
      <div v-else class="space-y-1.5">
        <div v-for="d in DENOMS" :key="d" class="flex items-center justify-between gap-2">
          <span class="num text-body">{{ d }}</span>
          <input v-model.number="counts[d]" type="number" min="0" class="control h-8 w-24" />
        </div>
      </div>

      <div
        class="flex items-center justify-between rounded-lg px-4 py-2.5 font-medium"
        :class="Math.abs(variance) < 0.01 ? 'bg-success/10 text-success' : variance > 0 ? 'bg-primary/10 text-primary' : 'bg-danger/10 text-danger'"
        data-testid="shift-variance"
      >
        <span>{{ Math.abs(variance) < 0.01 ? 'مطابق' : variance > 0 ? 'زيادة في الصندوق' : 'عجز في الصندوق' }}</span>
        <span class="num">{{ variance.toFixed(2) }}</span>
      </div>

      <div>
        <span class="field-label">عند الإغلاق</span>
        <SegmentedControl
          v-model="handoverMode"
          :options="[
            { value: 'HANDOVER', label: 'تسليم للوردية التالية' },
            { value: 'DROP', label: 'إيداع بالخزينة/البنك' },
          ]"
        />
      </div>
      <div>
        <label class="field-label" for="close-note">ملاحظة</label>
        <input id="close-note" v-model="note" class="control h-9" />
      </div>
    </div>
    <template #footer>
      <AppButton :disabled="submitting" @click="open = false">إلغاء</AppButton>
      <AppButton variant="primary" :loading="submitting" @click="confirm">إغلاق الوردية وطباعة تقرير Z</AppButton>
    </template>
  </AppModal>
</template>
