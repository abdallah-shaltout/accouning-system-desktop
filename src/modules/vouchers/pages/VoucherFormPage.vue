<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Save } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, todayKey } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { accountPath, getAccounts, type AccountWithBalance } from '@/modules/accounting/services/accountingService';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { createOwnerVoucher, createPaymentVoucher, createReceiptVoucher, createTransferVoucher } from '../services/voucherService';
import type { OwnerDirection, VoucherKind } from '../types';

const router = useRouter();
const route = useRoute();
const toast = useToast();
const settings = useSettingsStore();

const kind = ref<VoucherKind>((route.query.kind as VoucherKind) || 'RECEIPT');
const date = ref(todayKey());
const amount = ref<number>();
const description = ref('');
const note = ref('');
const paymentMethodId = ref<string | undefined>();
const otherAccountId = ref<string | undefined>();
const sourceAccountId = ref<string | undefined>();
const destinationAccountId = ref<string | undefined>();
const feeAmount = ref<number>();
const feeAccountId = ref<string | undefined>();
const ownerDirection = ref<OwnerDirection>('drawings');
const cashAccountId = ref<string | undefined>();
const accounts = ref<AccountWithBalance[]>([]);
const saving = ref(false);
const submitted = ref(false);

onMounted(async () => {
  await settings.load();
  accounts.value = await getAccounts();
  paymentMethodId.value = settings.paymentMethods.find((m) => m.active && m.showInPayments)?.id;
  cashAccountId.value = accounts.value.find((a) => a.systemRole === 'cash')?.id;
});

const kindOptions: { value: VoucherKind; label: string }[] = [
  { value: 'RECEIPT', label: 'سند قبض عام' },
  { value: 'PAYMENT', label: 'سند صرف عام' },
  { value: 'TRANSFER', label: 'تحويل بين الحسابات' },
  { value: 'OWNER', label: 'مسحوبات / رأس مال' },
];

const methodOptions = computed(() => settings.paymentMethods.filter((m) => m.active && m.showInPayments).map((m) => ({ value: m.id, label: m.name })));
const accountOptions = computed(() =>
  accounts.value.filter((a) => a.active && !a.isGroup && a.allowManual).map((a) => ({ value: a.id, label: `${a.code} — ${a.name}`, sublabel: accountPath(a, accounts.value) })),
);
const cashBankOptions = computed(() =>
  accounts.value.filter((a) => a.active && (a.subtype === 'cash' || a.subtype === 'bank')).map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` })),
);

const problems = computed(() => {
  const list: string[] = [];
  if (!(num0(amount.value) > 0)) list.push('أدخل مبلغاً أكبر من صفر');
  if (!description.value.trim()) list.push('أدخل وصفاً للسند');
  if (kind.value === 'RECEIPT' && (!paymentMethodId.value || !otherAccountId.value)) list.push('اختر طريقة الدفع والحساب الدائن');
  if (kind.value === 'PAYMENT' && (!paymentMethodId.value || !otherAccountId.value)) list.push('اختر طريقة الدفع والحساب المدين');
  if (kind.value === 'TRANSFER' && (!sourceAccountId.value || !destinationAccountId.value)) list.push('اختر حسابي المصدر والوجهة');
  if (kind.value === 'TRANSFER' && sourceAccountId.value && sourceAccountId.value === destinationAccountId.value) list.push('اختر حسابين مختلفين');
  if (kind.value === 'OWNER' && !cashAccountId.value) list.push('اختر الحساب النقدي/البنكي');
  return list;
});

async function save() {
  submitted.value = true;
  if (problems.value.length) return;
  saving.value = true;
  try {
    const base = { date: dateKeyToIso(date.value), amount: num0(amount.value), description: description.value.trim(), note: note.value.trim() || undefined };
    if (kind.value === 'RECEIPT') {
      const v = await createReceiptVoucher({ ...base, paymentMethodId: paymentMethodId.value!, creditAccountId: otherAccountId.value! });
      toast.success('تم تسجيل سند القبض العام', v.number);
    } else if (kind.value === 'PAYMENT') {
      const v = await createPaymentVoucher({ ...base, paymentMethodId: paymentMethodId.value!, debitAccountId: otherAccountId.value! });
      toast.success('تم تسجيل سند الصرف العام', v.number);
    } else if (kind.value === 'TRANSFER') {
      const v = await createTransferVoucher({
        ...base,
        sourceAccountId: sourceAccountId.value!,
        destinationAccountId: destinationAccountId.value!,
        feeAmount: feeAmount.value ? num0(feeAmount.value) : undefined,
        feeAccountId: feeAmount.value ? feeAccountId.value : undefined,
      });
      toast.success('تم تسجيل التحويل', v.number);
    } else {
      const v = await createOwnerVoucher({ ...base, direction: ownerDirection.value, cashAccountId: cashAccountId.value! });
      toast.success('تم تسجيل السند', v.number);
    }
    router.push('/vouchers');
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="سند عام جديد" back="/vouchers" />
    <div class="grid items-start gap-5 lg:grid-cols-[1fr_340px]">
      <AppCard padding="sm">
        <div class="space-y-4">
          <SegmentedControl v-model="kind" :options="kindOptions" />
          <div class="grid gap-4 sm:grid-cols-2">
            <AppInput v-model="date" type="date" label="التاريخ" required />
            <AppInput v-model.number="amount" type="number" min="0" step="0.01" label="المبلغ" required />
          </div>

          <template v-if="kind === 'RECEIPT'">
            <AppSelect v-model="paymentMethodId" label="يُقبض عبر" :options="methodOptions" />
            <AppCombobox v-model="otherAccountId" label="الحساب الدائن" :options="accountOptions" placeholder="اختر الحساب…" />
          </template>
          <template v-else-if="kind === 'PAYMENT'">
            <AppCombobox v-model="otherAccountId" label="الحساب المدين" :options="accountOptions" placeholder="اختر الحساب…" />
            <AppSelect v-model="paymentMethodId" label="يُصرف عبر" :options="methodOptions" />
          </template>
          <template v-else-if="kind === 'TRANSFER'">
            <div class="grid gap-4 sm:grid-cols-2">
              <AppCombobox v-model="sourceAccountId" label="من حساب" :options="accountOptions" placeholder="المصدر…" />
              <AppCombobox v-model="destinationAccountId" label="إلى حساب" :options="accountOptions" placeholder="الوجهة…" />
            </div>
            <div class="grid gap-4 sm:grid-cols-2">
              <AppInput v-model.number="feeAmount" type="number" min="0" step="0.01" label="عمولة التحويل (اختياري)" />
              <AppCombobox v-if="feeAmount" v-model="feeAccountId" label="حساب العمولة" :options="accountOptions" placeholder="اختر…" />
            </div>
          </template>
          <template v-else>
            <SegmentedControl
              v-model="ownerDirection"
              :options="[
                { value: 'drawings', label: 'مسحوبات (خروج)' },
                { value: 'contribution', label: 'إضافة رأس مال (دخول)' },
              ]"
            />
            <AppSelect v-model="cashAccountId" label="الحساب النقدي/البنكي" :options="cashBankOptions" />
          </template>

          <AppInput v-model="description" label="الوصف" required />
          <AppTextarea v-model="note" label="ملاحظات" :rows="2" />
        </div>
      </AppCard>

      <AppCard title="الترحيل" padding="sm">
        <ul v-if="submitted && problems.length" class="mb-3 list-inside list-disc text-xs text-danger">
          <li v-for="p in problems" :key="p">{{ p }}</li>
        </ul>
        <AppButton variant="primary" block :icon="Save" :loading="saving" @click="save">حفظ السند</AppButton>
        <p class="mt-3 text-tiny leading-5 text-text-secondary">ينشئ قيداً محاسبياً عادياً مرتبطاً بهذا السند، ويمكن طباعته كسند PDF.</p>
      </AppCard>
    </div>
  </div>
</template>
