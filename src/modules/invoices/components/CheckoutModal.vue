<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Banknote, CreditCard, Landmark, NotebookPen } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import JournalPreview from '@/modules/core/components/JournalPreview.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { formatMoney } from '@/modules/core/helpers/format';
import { num0, toNum } from '@/modules/core/helpers/numbers';
import { changeDue, round2 } from '../helpers/totals';
import { previewSale } from '../services/invoiceService';
import type { JournalPreviewLine, SaleInput, SalePaymentMethod } from '../types';

const props = defineProps<{
  total: number;
  customerName?: string;
  /** Sale payload minus payment fields — used for the journal preview. */
  draft: Omit<SaleInput, 'paymentMethod' | 'paidAmount' | 'tenderedAmount'>;
  submitting?: boolean;
}>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ confirm: [payment: { paymentMethod: SalePaymentMethod; paidAmount: number; tenderedAmount?: number }] }>();

const method = ref<SalePaymentMethod>('cash');
const tendered = ref<number | undefined>();
const tenderInput = ref<HTMLInputElement>();
const preview = ref<JournalPreviewLine[]>([]);
const previewError = ref('');
const showJournal = ref(false);

const hasCustomer = computed(() => !!props.draft.customerId);
const methodOptions = computed(() => [
  { value: 'cash' as const, label: 'نقداً', icon: Banknote },
  { value: 'card' as const, label: 'بطاقة', icon: CreditCard },
  { value: 'bank_transfer' as const, label: 'تحويل', icon: Landmark },
  ...(hasCustomer.value ? [{ value: 'credit' as const, label: 'آجل', icon: NotebookPen }] : []),
]);

/** Quick-tender buttons: exact amount, then the next round banknotes above the total. */
const quickAmounts = computed(() => {
  const t = props.total;
  const set = new Set<number>([round2(t)]);
  for (const step of [10, 50, 100, 200, 500]) {
    const v = Math.ceil(t / step) * step;
    if (v > t) set.add(v);
  }
  return [...set].sort((a, b) => a - b).slice(0, 5);
});

const paid = computed(() => {
  if (method.value === 'card' || method.value === 'bank_transfer') return props.total;
  if (method.value === 'credit') return 0;
  return Math.min(num0(tendered.value), props.total);
});
const change = computed(() => (method.value === 'cash' ? changeDue(props.total, num0(tendered.value)) : 0));
const onAccount = computed(() => round2(props.total - paid.value));

const problem = computed(() => {
  if (method.value === 'cash' && toNum(tendered.value) === undefined) return 'أدخل المبلغ المستلم';
  if (onAccount.value > 0 && !hasCustomer.value) return 'المبلغ أقل من الإجمالي — اختر عميلاً للبيع الجزئي أو الآجل';
  return '';
});

watch(open, async (isOpen) => {
  if (!isOpen) return;
  method.value = 'cash';
  tendered.value = round2(props.total);
  showJournal.value = false;
  await nextTick();
  tenderInput.value?.focus();
  tenderInput.value?.select();
});

let previewTimer: ReturnType<typeof setTimeout> | undefined;
watch(
  [open, method, paid],
  () => {
    if (!open.value) return;
    clearTimeout(previewTimer);
    previewTimer = setTimeout(async () => {
      try {
        preview.value = await previewSale({ ...props.draft, paymentMethod: method.value, paidAmount: paid.value });
        previewError.value = '';
      } catch (err) {
        previewError.value = err instanceof Error ? err.message : String(err);
      }
    }, 200);
  },
  { immediate: true },
);

function confirm() {
  if (problem.value || props.submitting) return;
  emit('confirm', {
    paymentMethod: method.value,
    paidAmount: paid.value,
    tenderedAmount: method.value === 'cash' ? toNum(tendered.value) : undefined,
  });
}
</script>

<template>
  <AppModal v-model:open="open" title="الدفع" :description="customerName ? `العميل: ${customerName}` : 'عميل نقدي'" :persistent="submitting">
    <div class="space-y-5" @keydown.enter.prevent="confirm">
      <div class="rounded-lg border border-border bg-surface px-4 py-3 text-center">
        <p class="text-xs text-text-secondary">المطلوب</p>
        <p class="mt-0.5 text-3xl font-semibold tracking-tight"><MoneyText :value="total" /></p>
      </div>

      <div>
        <span class="field-label">طريقة الدفع</span>
        <SegmentedControl v-model="method" :options="methodOptions" />
      </div>

      <div v-if="method === 'cash'" class="space-y-3">
        <label class="field-label" for="tendered">المبلغ المستلم</label>
        <input
          id="tendered"
          ref="tenderInput"
          v-model.number="tendered"
          type="number"
          min="0"
          step="0.01"
          class="control h-12 text-xl font-medium"
          @focus="($event.target as HTMLInputElement).select()"
        />
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="a in quickAmounts"
            :key="a"
            type="button"
            class="num h-8 rounded-md border border-border px-3 text-[13px] hover:bg-surface-hover"
            :class="tendered === a && 'border-primary text-primary'"
            @click="tendered = a"
          >
            {{ formatMoney(a) }}
          </button>
        </div>
        <div class="flex items-center justify-between rounded-lg bg-success/10 px-4 py-2.5" :class="change === 0 && 'opacity-60'">
          <span class="text-[13px]">الباقي للعميل</span>
          <span class="text-xl font-semibold text-success"><MoneyText :value="change" /></span>
        </div>
      </div>

      <p v-else-if="method === 'credit'" class="rounded-md bg-warning/10 px-3 py-2 text-[13px] text-warning">
        سيُسجل كامل المبلغ على حساب العميل ويظهر في الفواتير غير المسددة.
      </p>
      <p v-else class="rounded-md bg-surface px-3 py-2 text-[13px] text-text-secondary">
        {{ method === 'card' ? 'مرر البطاقة على جهاز نقاط البيع ثم أكّد الدفع.' : 'تأكد من وصول التحويل قبل التأكيد.' }}
      </p>

      <p v-if="onAccount > 0 && hasCustomer && method === 'cash'" class="rounded-md bg-warning/10 px-3 py-2 text-[13px] text-warning">
        دفع جزئي — سيُسجل المتبقي <MoneyText :value="onAccount" /> على حساب العميل.
      </p>
      <p v-if="problem" class="text-xs text-danger">{{ problem }}</p>

      <div>
        <button type="button" class="text-xs text-text-secondary hover:text-text-primary" @click="showJournal = !showJournal">
          {{ showJournal ? '▾' : '◂' }} القيد المحاسبي الذي سيُسجل
        </button>
        <div v-if="showJournal" class="mt-2">
          <p v-if="previewError" class="text-xs text-danger">{{ previewError }}</p>
          <JournalPreview v-else :lines="preview" />
        </div>
      </div>
    </div>
    <template #footer>
      <AppButton :disabled="submitting" @click="open = false">رجوع</AppButton>
      <AppButton variant="primary" size="lg" :loading="submitting" :disabled="!!problem" kbd="Enter" @click="confirm">تأكيد البيع</AppButton>
    </template>
  </AppModal>
</template>
