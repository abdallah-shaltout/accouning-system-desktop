<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §1 "F10 = pay-in/pay-out", §5 "bank drop"). */
import { ref, watch } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { num0 } from '@/modules/core/helpers/numbers';

const open = defineModel<boolean>('open', { default: false });
const props = defineProps<{ submitting?: boolean }>();
const emit = defineEmits<{ confirm: [kind: 'PAY_IN' | 'PAY_OUT' | 'BANK_DROP', amount: number, note?: string] }>();

const kind = ref<'PAY_IN' | 'PAY_OUT' | 'BANK_DROP'>('PAY_OUT');
const amount = ref<number | undefined>();
const note = ref('');

watch(open, (o) => {
  if (o) {
    kind.value = 'PAY_OUT';
    amount.value = undefined;
    note.value = '';
  }
});

function confirm() {
  if (!(num0(amount.value) > 0)) return;
  emit('confirm', kind.value, num0(amount.value), note.value.trim() || undefined);
}
</script>

<template>
  <AppModal v-model:open="open" title="إيداع / سحب من الدرج" size="sm" :persistent="submitting">
    <div class="space-y-4">
      <SegmentedControl
        v-model="kind"
        :options="[
          { value: 'PAY_IN', label: 'إيداع (Pay in)' },
          { value: 'PAY_OUT', label: 'سحب (Pay out)' },
          { value: 'BANK_DROP', label: 'إيداع بالبنك/الخزينة' },
        ]"
      />
      <div>
        <label class="field-label" for="cashinout-amount">المبلغ</label>
        <input id="cashinout-amount" v-model.number="amount" type="number" min="0" step="0.01" class="control h-11 text-lg" autofocus />
      </div>
      <div>
        <label class="field-label" for="cashinout-note">ملاحظة</label>
        <input id="cashinout-note" v-model="note" class="control h-9" placeholder="مثال: أجرة توصيل، مصروف طارئ..." />
      </div>
    </div>
    <template #footer>
      <AppButton :disabled="submitting" @click="open = false">إلغاء</AppButton>
      <AppButton variant="primary" :loading="submitting" :disabled="!(num0(amount) > 0)" @click="confirm">تأكيد</AppButton>
    </template>
  </AppModal>
</template>
