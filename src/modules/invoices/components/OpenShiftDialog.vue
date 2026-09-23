<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §5 "Open: pick the branch and cash drawer, then count the opening float (optionally by denomination)"). */
import { computed, ref, watch } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import { num0 } from '@/modules/core/helpers/numbers';
import type { DenominationCount } from '../types';

const DENOMS = [500, 200, 100, 50, 20, 10, 5, 1];

const open = defineModel<boolean>('open', { default: false });
const props = defineProps<{ submitting?: boolean }>();
const emit = defineEmits<{ confirm: [openingFloat: number, denominations?: DenominationCount[]] }>();

const useDenoms = ref(false);
const counts = ref<Record<number, number>>({});
const flatFloat = ref<number | undefined>();

watch(open, (o) => {
  if (o) {
    counts.value = Object.fromEntries(DENOMS.map((d) => [d, 0]));
    flatFloat.value = undefined;
    useDenoms.value = false;
  }
});

const denomTotal = computed(() => DENOMS.reduce((a, d) => a + d * num0(counts.value[d]), 0));
const total = computed(() => (useDenoms.value ? denomTotal.value : num0(flatFloat.value)));

function confirm() {
  if (total.value < 0) return;
  const denoms: DenominationCount[] | undefined = useDenoms.value ? DENOMS.map((value) => ({ value, count: num0(counts.value[value]) })) : undefined;
  emit('confirm', total.value, denoms);
}
</script>

<template>
  <AppModal v-model:open="open" title="فتح وردية" size="sm" :persistent="submitting">
    <div class="space-y-4">
      <label class="flex items-center gap-2 text-body">
        <input v-model="useDenoms" type="checkbox" class="size-4" />
        عدّ الرصيد الافتتاحي حسب الفئات
      </label>

      <div v-if="!useDenoms">
        <label class="field-label" for="opening-float">الرصيد الافتتاحي</label>
        <input id="opening-float" v-model.number="flatFloat" type="number" min="0" step="0.01" class="control h-11 text-lg" autofocus />
      </div>
      <div v-else class="space-y-1.5">
        <div v-for="d in DENOMS" :key="d" class="flex items-center justify-between gap-2">
          <span class="num text-body">{{ d }}</span>
          <input v-model.number="counts[d]" type="number" min="0" class="control h-8 w-24" />
        </div>
      </div>

      <div class="flex items-center justify-between rounded-lg bg-surface px-4 py-2.5 font-medium">
        <span>الإجمالي</span>
        <span class="num">{{ total.toFixed(2) }}</span>
      </div>
    </div>
    <template #footer>
      <AppButton :disabled="submitting" @click="open = false">إلغاء</AppButton>
      <AppButton variant="primary" :loading="submitting" :disabled="total < 0" @click="confirm">فتح الوردية</AppButton>
    </template>
  </AppModal>
</template>
