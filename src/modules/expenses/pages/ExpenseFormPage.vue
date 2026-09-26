<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Save } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, todayKey } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { getSuppliers } from '@/modules/parties/services/partyService';
import type { Supplier } from '@/modules/parties/types';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { createExpense, getExpenseCategories } from '../services/expenseService';
import type { ExpenseCategory } from '../types';

const router = useRouter();
const route = useRoute();
const toast = useToast();
const settings = useSettingsStore();

const categories = ref<ExpenseCategory[]>([]);
const suppliers = ref<Supplier[]>([]);
const loading = ref(true);

const date = ref(todayKey());
const categoryId = ref<string | undefined>(typeof route.query.category === 'string' ? route.query.category : undefined);
const amount = ref<number>();
const isTaxInvoice = ref(false);
const taxId = ref<string | undefined>();
const supplierVatNumber = ref('');
const supplierInvoiceNo = ref('');
const paidKind = ref<'method' | 'credit'>('method');
const paymentMethodId = ref<string | undefined>();
const supplierId = ref<string | undefined>();
const description = ref('');
const repeatMonthly = ref(false);
const saving = ref(false);
const submitted = ref(false);
const draftOwnerRef = `expense:new:${Date.now()}`;

onMounted(async () => {
  await settings.load();
  [categories.value, suppliers.value] = await Promise.all([getExpenseCategories(), getSuppliers()]);
  taxId.value = settings.taxes.find((t) => t.direction === 'purchase' && t.isDefault)?.id;
  const method = settings.paymentMethods.find((m) => m.showInPayments && m.type !== 'credit' && m.type !== 'store_credit');
  paymentMethodId.value = method?.id;
  loading.value = false;
});

const categoryOptions = computed(() => categories.value.filter((c) => c.active).map((c) => ({ value: c.id, label: c.name })));
const methodOptions = computed(() => settings.paymentMethods.filter((m) => m.active && m.type !== 'credit' && m.type !== 'store_credit').map((m) => ({ value: m.id, label: m.name })));
const supplierOptions = computed(() => suppliers.value.map((s) => ({ value: s.id, label: s.name })));
const purchaseTaxOptions = computed(() => settings.taxes.filter((t) => t.direction === 'purchase').map((t) => ({ value: t.id, label: `${t.name} (${t.rate}%)` })));

const taxRate = computed(() => settings.taxes.find((t) => t.id === taxId.value)?.rate ?? 0);
const netAmount = computed(() => (isTaxInvoice.value ? num0(amount.value) / (1 + taxRate.value / 100) : num0(amount.value)));
const vatAmount = computed(() => (isTaxInvoice.value ? num0(amount.value) - netAmount.value : 0));

const problems = computed(() => {
  const list: string[] = [];
  if (!categoryId.value) list.push('اختر تصنيف المصروف');
  if (!(num0(amount.value) > 0)) list.push('أدخل مبلغاً أكبر من صفر');
  if (paidKind.value === 'method' && !paymentMethodId.value) list.push('اختر طريقة الدفع');
  if (paidKind.value === 'credit' && !supplierId.value) list.push('اختر المورد');
  return list;
});

async function save() {
  submitted.value = true;
  if (problems.value.length) return;
  saving.value = true;
  try {
    const expense = await createExpense({
      date: dateKeyToIso(date.value),
      categoryId: categoryId.value!,
      amount: num0(amount.value),
      isTaxInvoice: isTaxInvoice.value,
      taxId: isTaxInvoice.value ? taxId.value : undefined,
      supplierVatNumber: isTaxInvoice.value ? supplierVatNumber.value.trim() || undefined : undefined,
      supplierInvoiceNo: isTaxInvoice.value ? supplierInvoiceNo.value.trim() || undefined : undefined,
      paidFrom: paidKind.value === 'method' ? { kind: 'method', paymentMethodId: paymentMethodId.value! } : { kind: 'credit', supplierId: supplierId.value! },
      description: description.value.trim() || undefined,
      repeatMonthly: repeatMonthly.value,
    });
    toast.success('تم تسجيل المصروف', expense.number);
    router.push({ name: 'expense-detail', params: { id: expense.id } });
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="مصروف جديد" :back="{ name: 'expenses' }" />
    <div v-if="loading" class="text-body text-text-secondary">جارِ التحميل…</div>
    <div v-else class="grid items-start gap-5 lg:grid-cols-[1fr_340px]">
      <AppCard padding="sm">
        <div class="space-y-4">
          <div class="grid gap-4 sm:grid-cols-2">
            <AppDatePicker v-model="date" label="التاريخ" required />
            <AppCombobox v-model="categoryId" label="التصنيف" required :options="categoryOptions" :error="submitted && !categoryId ? 'اختر التصنيف' : undefined" />
          </div>
          <AppInput v-model.number="amount" type="number" min="0" step="0.01" label="المبلغ" required />

          <AppSwitch v-model="isTaxInvoice" label="فاتورة ضريبية؟" description="يتم فصل الضريبة عن المبلغ وتُحتسب ضريبة مدخلات قابلة للاسترداد" />
          <template v-if="isTaxInvoice">
            <div class="grid gap-4 sm:grid-cols-2">
              <AppSelect v-model="taxId" label="نوع الضريبة" :options="purchaseTaxOptions" />
              <AppInput v-model="supplierVatNumber" label="الرقم الضريبي للمورد" ltr />
            </div>
            <AppInput v-model="supplierInvoiceNo" label="رقم فاتورة المورد" ltr />
            <p class="text-tiny text-text-secondary">صافي <span class="num">{{ netAmount.toFixed(2) }}</span> + ضريبة <span class="num">{{ vatAmount.toFixed(2) }}</span></p>
          </template>

          <div>
            <span class="field-label">الدفع من</span>
            <SegmentedControl
              v-model="paidKind"
              :options="[
                { value: 'method', label: 'طريقة دفع / صندوق' },
                { value: 'credit', label: 'آجل لمورد' },
              ]"
            />
          </div>
          <AppSelect v-if="paidKind === 'method'" v-model="paymentMethodId" label="طريقة الدفع" :options="methodOptions" :error="submitted && !paymentMethodId ? 'اختر طريقة الدفع' : undefined" />
          <AppCombobox v-else v-model="supplierId" label="المورد" :options="supplierOptions" :error="submitted && !supplierId ? 'اختر المورد' : undefined" />

          <AppTextarea v-model="description" label="الوصف" :rows="2" />
          <AppSwitch v-model="repeatMonthly" label="تكرار شهري" description="يمكن إنشاء قالب متكرر لاحقاً من صفحة المصروفات المتكررة" />
          <div>
            <span class="field-label">المرفقات (صورة الفاتورة)</span>
            <AttachmentField :owner-ref="draftOwnerRef" />
          </div>
        </div>
      </AppCard>

      <AppCard title="الترحيل" padding="sm">
        <ul v-if="submitted && problems.length" class="mb-3 list-inside list-disc text-xs text-danger">
          <li v-for="p in problems" :key="p">{{ p }}</li>
        </ul>
        <AppButton variant="primary" block :icon="Save" :loading="saving" @click="save">حفظ المصروف</AppButton>
        <p class="mt-3 text-tiny leading-5 text-text-secondary">القيد: من حساب المصروف (+ضريبة المدخلات إن وُجدت) إلى حساب طريقة الدفع أو حساب الموردين.</p>
      </AppCard>
    </div>
  </div>
</template>
