<script setup lang="ts">
/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §1 "Line discount: % or amount (F8 on the selected line;
 * Shift+F8 = invoice discount). Over the user's maxDiscountPct → a manager PIN approval overlay,
 * which is logged"). One dialog handles both line and invoice discounts — `kind` picks the copy.
 */
import { computed, ref, watch } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { num0, toNum } from '@/modules/core/helpers/numbers';
import { verifyManagerPin } from '@/modules/users/services/authService';

const props = defineProps<{ kind: 'line' | 'invoice'; maxPct: number; initialValue?: number; initialIsPct?: boolean }>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ apply: [value: number, isPct: boolean, approvedBy?: string] }>();

const mode = ref<'pct' | 'amount'>('pct');
const value = ref<number | undefined>();
const pinUser = ref('');
const pinPass = ref('');
const pinError = ref('');
const pinBusy = ref(false);
const needsApproval = ref(false);

watch(open, (o) => {
  if (o) {
    mode.value = props.initialIsPct === false ? 'amount' : 'pct';
    value.value = props.initialValue || undefined;
    needsApproval.value = false;
    pinUser.value = '';
    pinPass.value = '';
    pinError.value = '';
  }
});

const overLimit = computed(() => mode.value === 'pct' && num0(value.value) > props.maxPct);
const problem = computed(() => {
  if (toNum(value.value) === undefined || num0(value.value) < 0) return 'أدخل قيمة صحيحة';
  if (mode.value === 'pct' && num0(value.value) > 100) return 'النسبة لا يمكن أن تتجاوز 100%';
  return '';
});

function apply() {
  if (problem.value) return;
  if (overLimit.value && !needsApproval.value) {
    needsApproval.value = true;
    return;
  }
  emit('apply', num0(value.value), mode.value === 'pct');
  open.value = false;
}

async function submitPin() {
  pinError.value = '';
  if (!pinUser.value.trim() || !pinPass.value) {
    pinError.value = 'أدخل اسم المستخدم وكلمة المرور';
    return;
  }
  pinBusy.value = true;
  try {
    const manager = await verifyManagerPin(pinUser.value, pinPass.value);
    emit('apply', num0(value.value), mode.value === 'pct', manager.id);
    open.value = false;
  } catch (err) {
    pinError.value = errorMessage(err);
  } finally {
    pinBusy.value = false;
  }
}
</script>

<template>
  <AppModal v-model:open="open" :title="kind === 'line' ? 'خصم على الصنف' : 'خصم على الفاتورة'" size="sm">
    <div v-if="!needsApproval" class="space-y-3">
      <SegmentedControl v-model="mode" :options="[{ value: 'pct', label: '%' }, { value: 'amount', label: 'مبلغ' }]" />
      <input v-model.number="value" type="number" min="0" step="0.01" class="control h-11 text-lg" autofocus @focus="($event.target as HTMLInputElement).select()" @keydown.enter="apply" />
      <p v-if="mode === 'pct'" class="text-xs text-text-secondary">الحد المسموح لك بدون اعتماد: <span class="num">{{ maxPct }}</span>%</p>
      <p v-if="problem" class="text-xs text-danger">{{ problem }}</p>
    </div>
    <div v-else class="space-y-3">
      <p class="rounded-md bg-warning/10 px-3 py-2 text-body text-warning">الخصم يتجاوز الحد المسموح — يلزم اعتماد مدير.</p>
      <input v-model="pinUser" class="control h-10" placeholder="اسم مستخدم المدير" ltr autofocus />
      <input v-model="pinPass" type="password" class="control h-10" placeholder="كلمة المرور" ltr @keydown.enter="submitPin" />
      <p v-if="pinError" class="text-xs text-danger">{{ pinError }}</p>
    </div>
    <template #footer>
      <AppButton @click="open = false">إلغاء</AppButton>
      <AppButton v-if="!needsApproval" variant="primary" :disabled="!!problem" @click="apply">تطبيق</AppButton>
      <AppButton v-else variant="primary" :loading="pinBusy" @click="submitPin">اعتماد وتطبيق</AppButton>
    </template>
  </AppModal>
</template>
