<script setup lang="ts">
/**
 * VAT settlement (docs/v2/11-journal-dashboard-insights.md A4, 02-accounting-review.md §3): shows
 * output/input VAT and the net for a period, posts the settlement entry, then offers "سداد" (pay).
 * The payment posting routes through a real payment method via `recordPaymentVoucher` (docs/v2/09
 * §2's shared voucher helper) — same as every other cash-out flow — instead of hard-coding
 * cash/bank, closing the follow-up `payVatSettlement` used to carry since Phase 3.
 */
import { computed, onMounted, ref, watch } from 'vue';
import { CircleCheck, Receipt } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { startOfMonthKey, todayKey } from '@/modules/core/helpers/format';
import { getPaymentMethods } from '@/modules/settings/services/settingsService';
import type { PaymentMethod } from '@/modules/settings/types';
import { getVatPeriodTotals, payVatSettlementNow, submitVatSettlement } from '../services/accountingService';

const toast = useToast();
const confirm = useConfirm();

const paymentMethods = ref<PaymentMethod[]>([]);
const payMethodId = ref<string | undefined>();
onMounted(async () => {
  paymentMethods.value = (await getPaymentMethods()).filter((m) => m.showInPayments);
  payMethodId.value = paymentMethods.value[0]?.id;
});

const from = ref(startOfMonthKey());
const to = ref(todayKey());

const { data, loading, error, reload } = useAsync(() => getVatPeriodTotals(from.value, to.value));
watch([from, to], reload);

const posting = ref(false);
const lastEntryId = ref<string | null>(null);

async function postSettlement() {
  const ok = await confirm({
    title: 'ترحيل قيد تسوية ضريبة القيمة المضافة؟',
    message: 'سيُقفل حسابا ضريبة المخرجات والمدخلات وتُنقل الفروقات إلى حساب صافي الضريبة المستحقة.',
    confirmText: 'ترحيل القيد',
  });
  if (!ok) return;
  posting.value = true;
  try {
    const entry = await submitVatSettlement(from.value, to.value);
    lastEntryId.value = entry.id;
    toast.success('تم ترحيل قيد التسوية', entry.number);
    payOpen.value = true;
  } catch (err) {
    toast.error(err);
  } finally {
    posting.value = false;
  }
}

// --- سداد (pay) ----------------------------------------------------------------------------

const payOpen = ref(false);
const paying = ref(false);

async function pay() {
  if (!data.value || !payMethodId.value) return;
  const amount = Math.max(0, data.value.net);
  paying.value = true;
  try {
    const entry = await payVatSettlementNow(amount, payMethodId.value);
    toast.success('تم تسجيل سداد الضريبة', entry.number);
    payOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    paying.value = false;
  }
}

const netLabel = computed(() => {
  if (!data.value) return '';
  return data.value.net > 0 ? 'صافي مستحق للمصلحة' : data.value.net < 0 ? 'صافي قابل للاسترداد' : 'لا يوجد صافي مستحق';
});
</script>

<template>
  <div>
    <PageHeader title="تسوية ضريبة القيمة المضافة" subtitle="ضريبة المخرجات والمدخلات وصافي المستحق لفترة محددة" />

    <div class="mb-4">
      <DateRangeFilter v-model:from="from" v-model:to="to" />
    </div>

    <div v-if="error" class="rounded-xl border border-danger/30 bg-danger/5 p-4 text-body text-danger">{{ error }}</div>
    <div v-else class="grid gap-4 sm:grid-cols-3">
      <AppCard padding="sm">
        <p class="text-xs text-text-secondary">ضريبة المخرجات (على المبيعات)</p>
        <p class="num mt-2 text-2xl font-semibold">
          <MoneyText v-if="!loading && data" :value="data.outputVat" plain />
          <span v-else class="inline-block h-6 w-24 animate-shimmer rounded bg-surface-hover" />
        </p>
      </AppCard>
      <AppCard padding="sm">
        <p class="text-xs text-text-secondary">ضريبة المدخلات (على المشتريات)</p>
        <p class="num mt-2 text-2xl font-semibold">
          <MoneyText v-if="!loading && data" :value="data.inputVat" plain />
          <span v-else class="inline-block h-6 w-24 animate-shimmer rounded bg-surface-hover" />
        </p>
      </AppCard>
      <AppCard padding="sm" class="border-primary/30 bg-primary/5">
        <p class="text-xs text-text-secondary">{{ netLabel }}</p>
        <p class="num mt-2 text-2xl font-semibold" :class="data && data.net < 0 && 'text-success'">
          <MoneyText v-if="!loading && data" :value="Math.abs(data.net)" plain />
          <span v-else class="inline-block h-6 w-24 animate-shimmer rounded bg-surface-hover" />
        </p>
      </AppCard>
    </div>

    <div class="mt-6 flex flex-wrap gap-2">
      <AppButton variant="primary" :icon="Receipt" :loading="posting" :disabled="!data || (data.outputVat === 0 && data.inputVat === 0)" @click="postSettlement">
        ترحيل قيد التسوية
      </AppButton>
      <AppButton v-if="data && data.net > 0" @click="payOpen = true">سداد الضريبة</AppButton>
      <AppButton v-if="lastEntryId" variant="ghost" :to="{ name: 'journal-entry', params: { id: lastEntryId } }">عرض آخر قيد تسوية</AppButton>
    </div>

    <AppModal v-model:open="payOpen" title="سداد ضريبة القيمة المضافة" size="sm">
      <div class="space-y-4">
        <p class="text-body">
          المبلغ المستحق: <MoneyText :value="data ? Math.max(0, data.net) : 0" />
        </p>
        <AppSelect v-model="payMethodId" label="طريقة السداد" :options="paymentMethods.map((m) => ({ value: m.id, label: m.name }))" />
        <p class="flex items-center gap-1.5 text-xs text-text-secondary">
          <CircleCheck class="size-3.5" /> سيُنشأ سند صرف (مدين صافي الضريبة المستحقة / دائن حساب طريقة السداد المختارة).
        </p>
      </div>
      <template #footer>
        <AppButton @click="payOpen = false">إغلاق</AppButton>
        <AppButton variant="primary" :disabled="!payMethodId" :loading="paying" @click="pay">تأكيد السداد</AppButton>
      </template>
    </AppModal>
  </div>
</template>
