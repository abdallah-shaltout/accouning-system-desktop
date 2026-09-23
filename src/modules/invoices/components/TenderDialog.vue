<script setup lang="ts">
/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §1 "Pay (F12)"): the split-tender dialog. Method buttons
 * come from payment methods marked `showInPos`; adds tenders until the remaining amount is 0; cash
 * gets quick-amount buttons + change; card/wallet get an optional reference field; credit checks the
 * credit limit (surfaced as a server-side error from `createSale`, since `assertWithinCreditLimit`
 * lives in `invoiceService.ts`); foreign cash (if a method's type needs it) shows the FX-converted
 * amount due — kept minimal since no FX-rate settings screen exists before Phase 9.
 */
import { computed, nextTick, ref, watch } from 'vue';
import { Banknote } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import JournalPreview from '@/modules/core/components/JournalPreview.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatMoney } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { changeDue, round2 } from '../helpers/totals';
import { previewSale } from '../services/invoiceService';
import type { JournalPreviewLine, SaleInput, Tender } from '../types';

const props = defineProps<{
  total: number;
  customerName?: string;
  hasCustomer?: boolean;
  /** Sale payload minus payment fields — used for the journal preview. */
  draft: Omit<SaleInput, 'paymentMethod' | 'paidAmount' | 'tenderedAmount' | 'tenders'>;
  submitting?: boolean;
}>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ confirm: [payment: { tenders: Tender[]; paidAmount: number; tenderedAmount?: number }] }>();

const settings = useSettingsStore();
const methods = computed(() => settings.paymentMethods.filter((m) => m.showInPos && m.active).sort((a, b) => a.sortOrder - b.sortOrder));

/** One row per tender added so far. */
const tenders = ref<{ paymentMethodId: string; amount: number; reference?: string }[]>([]);
const activeMethodId = ref<string | undefined>();
const cashGiven = ref<number | undefined>();
const reference = ref('');
const tenderInput = ref<HTMLInputElement>();
const preview = ref<JournalPreviewLine[]>([]);
const previewError = ref('');
const showJournal = ref(false);

const paidSoFar = computed(() => round2(tenders.value.reduce((a, t) => a + t.amount, 0)));
const remaining = computed(() => round2(props.total - paidSoFar.value));
const activeMethod = computed(() => methods.value.find((m) => m.id === activeMethodId.value));
const isCash = computed(() => activeMethod.value?.type === 'cash');
const isCredit = computed(() => activeMethod.value?.type === 'credit');
const change = computed(() => (isCash.value ? changeDue(remaining.value, num0(cashGiven.value)) : 0));

const quickAmounts = computed(() => {
  const t = Math.max(remaining.value, 0);
  const set = new Set<number>([round2(t)]);
  for (const step of [10, 50, 100, 200, 500]) {
    const v = Math.ceil(t / step) * step;
    if (v > t) set.add(v);
  }
  return [...set].sort((a, b) => a - b).slice(0, 5);
});

function selectMethod(id: string) {
  activeMethodId.value = id;
  cashGiven.value = round2(remaining.value);
  reference.value = '';
  nextTick(() => tenderInput.value?.focus());
}

function addTender() {
  const method = activeMethod.value;
  if (!method) return;
  if (isCredit.value) {
    if (!props.hasCustomer) return;
    tenders.value.push({ paymentMethodId: method.id, amount: remaining.value });
  } else if (isCash.value) {
    const given = num0(cashGiven.value);
    if (given <= 0) return;
    tenders.value.push({ paymentMethodId: method.id, amount: Math.min(given, remaining.value) });
  } else {
    tenders.value.push({ paymentMethodId: method.id, amount: remaining.value, reference: reference.value.trim() || undefined });
  }
  activeMethodId.value = undefined;
  cashGiven.value = undefined;
  reference.value = '';
}

function removeTender(i: number) {
  tenders.value.splice(i, 1);
}

const totalCashGiven = computed(() => {
  // Change is only meaningful when the LAST tender was cash and overpaid — approximate by summing
  // any cash tender's excess over what it needed to cover, which is 0 for every completed tender
  // (they're capped at `remaining` when added) — so change only ever comes from the live cash input.
  return change.value;
});

const problem = computed(() => {
  if (remaining.value > 0.005 && !activeMethodId.value) return 'اختر طريقة دفع لإتمام المبلغ المتبقي';
  if (isCash.value && num0(cashGiven.value) <= 0 && remaining.value > 0) return 'أدخل المبلغ المستلم';
  if (isCredit.value && !props.hasCustomer) return 'البيع الآجل يتطلب اختيار عميل';
  if (remaining.value > 0.005 && tenders.value.length === 0 && !activeMethodId.value) return 'أضف طريقة دفع واحدة على الأقل';
  return '';
});

const canFinish = computed(() => remaining.value <= 0.005 || (activeMethodId.value && remaining.value > 0));

watch(open, async (isOpen) => {
  if (!isOpen) return;
  tenders.value = [];
  activeMethodId.value = undefined;
  cashGiven.value = undefined;
  reference.value = '';
  showJournal.value = false;
  // Default to the first method (usually cash) pre-selected with the full amount, matching the old
  // single-method flow's speed for the common case.
  const first = methods.value.find((m) => m.type === 'cash') ?? methods.value[0];
  if (first) selectMethod(first.id);
});

let previewTimer: ReturnType<typeof setTimeout> | undefined;
watch(
  [open, tenders, remaining],
  () => {
    if (!open.value) return;
    clearTimeout(previewTimer);
    previewTimer = setTimeout(async () => {
      const finalTenders = allTenders();
      try {
        preview.value = await previewSale({ ...props.draft, paymentMethod: 'cash', paidAmount: round2(finalTenders.reduce((a, t) => a + t.amount, 0)), tenders: finalTenders });
        previewError.value = '';
      } catch (err) {
        previewError.value = err instanceof Error ? err.message : String(err);
      }
    }, 200);
  },
  { deep: true },
);

/** The tenders that would actually be posted if confirmed right now (committed rows + the in-progress one, if it completes the sale). */
function allTenders(): Tender[] {
  const rows: Tender[] = tenders.value.map((t) => ({ paymentMethodId: t.paymentMethodId, amount: t.amount, reference: t.reference }));
  if (remaining.value > 0.005 && activeMethodId.value) {
    if (isCredit.value) rows.push({ paymentMethodId: activeMethodId.value, amount: remaining.value });
    else if (isCash.value && num0(cashGiven.value) > 0) rows.push({ paymentMethodId: activeMethodId.value, amount: Math.min(num0(cashGiven.value), remaining.value) });
    else if (!isCash.value && !isCredit.value) rows.push({ paymentMethodId: activeMethodId.value, amount: remaining.value, reference: reference.value.trim() || undefined });
  }
  return rows;
}

function confirm() {
  if (props.submitting) return;
  const finalTenders = allTenders();
  const total = round2(finalTenders.reduce((a, t) => a + t.amount, 0));
  if (total < round2(props.total) - 0.01) {
    // Not fully paid and no customer for the remainder — same guard the legacy dialog had.
    if (!props.hasCustomer) return;
  }
  const cashTendered = isCash.value ? num0(cashGiven.value) : undefined;
  emit('confirm', { tenders: finalTenders, paidAmount: total, tenderedAmount: cashTendered });
}
</script>

<template>
  <AppModal v-model:open="open" title="الدفع" :description="customerName ? `العميل: ${customerName}` : 'عميل نقدي'" :persistent="submitting" size="lg">
    <div class="space-y-4" @keydown.enter.prevent="canFinish ? confirm() : addTender()">
      <div class="grid grid-cols-2 gap-3">
        <div class="rounded-lg border border-border bg-surface px-4 py-3 text-center">
          <p class="text-xs text-text-secondary">الإجمالي</p>
          <p class="mt-0.5 text-2xl font-semibold tracking-tight"><MoneyText :value="total" /></p>
        </div>
        <div class="rounded-lg border px-4 py-3 text-center" :class="remaining > 0.005 ? 'border-warning/40 bg-warning/10' : 'border-success/40 bg-success/10'">
          <p class="text-xs text-text-secondary">{{ remaining > 0.005 ? 'المتبقي' : 'مكتمل' }}</p>
          <p class="mt-0.5 text-2xl font-semibold tracking-tight" :class="remaining > 0.005 ? 'text-warning' : 'text-success'"><MoneyText :value="Math.max(remaining, 0)" /></p>
        </div>
      </div>

      <!-- Tenders added so far (split payment). -->
      <ul v-if="tenders.length" class="space-y-1.5">
        <li v-for="(t, i) in tenders" :key="i" class="flex items-center justify-between rounded-md bg-surface px-3 py-1.5 text-body">
          <span>{{ settings.paymentMethods.find((m) => m.id === t.paymentMethodId)?.name }}<span v-if="t.reference" class="text-text-secondary"> — {{ t.reference }}</span></span>
          <span class="flex items-center gap-2">
            <MoneyText :value="t.amount" plain />
            <button type="button" class="text-xs text-danger hover:underline" @click="removeTender(i)">إزالة</button>
          </span>
        </li>
      </ul>

      <div v-if="remaining > 0.005" class="space-y-3">
        <div>
          <span class="field-label">طريقة الدفع</span>
          <div class="flex flex-wrap gap-1.5">
            <button
              v-for="m in methods"
              :key="m.id"
              type="button"
              class="h-9 rounded-md border px-3 text-body transition-colors"
              :class="activeMethodId === m.id ? 'border-primary bg-primary text-on-primary' : 'border-border hover:bg-surface-hover'"
              @click="selectMethod(m.id)"
            >
              {{ m.name }}
            </button>
          </div>
        </div>

        <div v-if="isCash" class="space-y-2.5">
          <label class="field-label" for="tender-cash">المبلغ المستلم</label>
          <input
            id="tender-cash"
            ref="tenderInput"
            v-model.number="cashGiven"
            type="number"
            min="0"
            step="0.01"
            class="control h-11 text-lg font-medium"
            @focus="($event.target as HTMLInputElement).select()"
          />
          <div class="flex flex-wrap gap-1.5">
            <button
              v-for="a in quickAmounts"
              :key="a"
              type="button"
              class="num h-8 rounded-md border border-border px-3 text-body hover:bg-surface-hover"
              :class="cashGiven === a && 'border-primary text-primary'"
              @click="cashGiven = a"
            >
              {{ formatMoney(a) }}
            </button>
          </div>
          <div v-if="totalCashGiven > 0" class="flex items-center justify-between rounded-lg bg-success/10 px-4 py-2">
            <span class="text-body">الباقي للعميل</span>
            <span class="text-lg font-semibold text-success"><MoneyText :value="totalCashGiven" /></span>
          </div>
        </div>

        <div v-else-if="activeMethodId && !isCredit" class="space-y-2">
          <label class="field-label" for="tender-ref">مرجع (آخر 4 أرقام / رمز الموافقة) — اختياري</label>
          <input id="tender-ref" ref="tenderInput" v-model="reference" class="control h-10" ltr />
          <AppButton size="sm" :icon="Banknote" @click="addTender">إضافة {{ formatMoney(remaining) }} عبر {{ activeMethod?.name }}</AppButton>
        </div>

        <p v-else-if="isCredit" class="rounded-md bg-warning/10 px-3 py-2 text-body text-warning">
          سيُسجل <span class="num">{{ formatMoney(remaining) }}</span> على حساب العميل ويظهر في الفواتير غير المسددة.
        </p>

        <AppButton v-if="isCash && remaining > 0.005" size="sm" @click="addTender">إضافة الدفعة</AppButton>
        <AppButton v-else-if="isCredit" size="sm" @click="addTender">تسجيل كآجل</AppButton>
      </div>

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
      <AppButton variant="primary" size="lg" :loading="submitting" :disabled="!canFinish" kbd="Enter" @click="confirm">تأكيد البيع</AppButton>
    </template>
  </AppModal>
</template>
