<script setup lang="ts">
// TODO(phase 8): wire <AttachmentField> onto payments/vouchers (docs/v2/14-platform.md §5).
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Banknote, CircleCheck, CreditCard, Landmark, Save } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, formatDate, todayKey } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { getCustomers, getSuppliers } from '@/modules/parties/services/partyService';
import type { Customer, Supplier } from '@/modules/parties/types';
import { createPayment, getOpenDocuments } from '../services/paymentService';
import type { OpenDocument, PaymentMethod, PaymentType } from '../types';

const route = useRoute();
const router = useRouter();
const toast = useToast();

const type = ref<PaymentType>(route.query.type === 'PAID' ? 'PAID' : 'RECEIVED');
const partyId = ref<string | undefined>(typeof route.query.party === 'string' ? route.query.party : undefined);
const refId = ref<string | undefined>(typeof route.query.ref === 'string' ? route.query.ref : undefined);
const amount = ref<number | undefined>();
const method = ref<PaymentMethod>('cash');
const date = ref(todayKey());
const note = ref('');
const saving = ref(false);
const submitted = ref(false);

const customers = ref<Customer[]>([]);
const suppliers = ref<Supplier[]>([]);
const docs = ref<OpenDocument[]>([]);
const docsLoading = ref(false);

const isReceived = computed(() => type.value === 'RECEIVED');
const selectedDoc = computed(() => docs.value.find((d) => d.id === refId.value));

onMounted(async () => {
  [customers.value, suppliers.value] = await Promise.all([getCustomers(), getSuppliers()]);
  if (partyId.value) loadDocs(true);
});

const partyOptions = computed(() =>
  (isReceived.value ? customers.value : suppliers.value)
    .slice()
    .sort((a, b) => b.balance - a.balance)
    .map((p) => ({ value: p.id, label: p.name, sublabel: p.balance > 0 ? `الرصيد ${p.balance.toFixed(2)}` : 'لا يوجد رصيد', disabled: p.balance <= 0 })),
);

async function loadDocs(keepRef = false) {
  docs.value = [];
  if (!keepRef) refId.value = undefined;
  if (!partyId.value) return;
  docsLoading.value = true;
  try {
    docs.value = await getOpenDocuments(isReceived.value ? 'customer' : 'supplier', partyId.value);
    if (!docs.value.some((d) => d.id === refId.value)) refId.value = docs.value[0]?.id;
    amount.value = selectedDoc.value?.outstanding;
  } finally {
    docsLoading.value = false;
  }
}

watch(type, () => {
  partyId.value = undefined;
  docs.value = [];
  refId.value = undefined;
  method.value = type.value === 'PAID' ? 'bank_transfer' : 'cash';
});
watch(partyId, () => loadDocs());
watch(refId, () => (amount.value = selectedDoc.value?.outstanding));

const problems = computed(() => {
  const list: string[] = [];
  if (!partyId.value) list.push(isReceived.value ? 'اختر العميل' : 'اختر المورد');
  else if (!refId.value) list.push('اختر المستند المراد سداده');
  if (!(num0(amount.value) > 0)) list.push('أدخل مبلغاً أكبر من صفر');
  else if (selectedDoc.value && num0(amount.value) > selectedDoc.value.outstanding + 0.001) list.push('المبلغ أكبر من المتبقي على المستند');
  return list;
});

async function save() {
  submitted.value = true;
  if (problems.value.length) return;
  saving.value = true;
  try {
    const payment = await createPayment({
      date: dateKeyToIso(date.value),
      type: type.value,
      targetType: isReceived.value ? 'customer' : 'supplier',
      targetId: partyId.value!,
      targetRef: refId.value!,
      amount: num0(amount.value),
      method: method.value,
      note: note.value.trim() || undefined,
    });
    toast.success(isReceived.value ? 'تم تسجيل سند القبض' : 'تم تسجيل سند الصرف', payment.number);
    router.push({ path: '/payments', query: { highlight: payment.id } });
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

const methodOptions = [
  { value: 'cash' as const, label: 'نقداً', icon: Banknote },
  { value: 'card' as const, label: 'بطاقة', icon: CreditCard },
  { value: 'bank_transfer' as const, label: 'تحويل بنكي', icon: Landmark },
];
</script>

<template>
  <div>
    <PageHeader :title="isReceived ? 'سند قبض' : 'سند صرف'" :subtitle="isReceived ? 'تحصيل مبلغ من عميل مقابل فاتورة' : 'سداد مبلغ لمورد مقابل أمر شراء'" back="/payments" />

    <div class="grid items-start gap-5 lg:grid-cols-[1fr_340px]">
      <div class="space-y-5">
        <AppCard padding="sm">
          <div class="grid gap-4 sm:grid-cols-[auto_1fr]">
            <div>
              <span class="field-label">النوع</span>
              <SegmentedControl
                v-model="type"
                :options="[
                  { value: 'RECEIVED', label: 'قبض من عميل' },
                  { value: 'PAID', label: 'صرف لمورد' },
                ]"
              />
            </div>
            <AppCombobox
              v-model="partyId"
              :label="isReceived ? 'العميل' : 'المورد'"
              required
              :options="partyOptions"
              :placeholder="isReceived ? 'اختر العميل…' : 'اختر المورد…'"
              :error="submitted && !partyId ? 'مطلوب' : undefined"
            />
          </div>
        </AppCard>

        <AppCard :title="isReceived ? 'الفاتورة المراد سدادها' : 'أمر الشراء المراد سداده'" padding="none">
          <div v-if="docsLoading" class="p-4"><SkeletonBlock :lines="3" height="h-10" /></div>
          <EmptyState
            v-else-if="!partyId"
            compact
            :title="isReceived ? 'اختر عميلاً لعرض فواتيره المفتوحة' : 'اختر مورداً لعرض أوامره غير المسددة'"
          />
          <EmptyState v-else-if="!docs.length" compact :icon="CircleCheck" title="لا توجد مستندات مفتوحة" description="كل المستندات مسددة بالكامل" />
          <ul v-else class="divide-y divide-border">
            <li v-for="d in docs" :key="d.id">
              <label class="flex cursor-pointer items-center gap-3 px-4 py-3 hover:bg-surface-hover" :class="refId === d.id && 'bg-background'">
                <input v-model="refId" type="radio" :value="d.id" class="size-4 accent-[var(--color-primary)]" />
                <span class="flex-1">
                  <span class="num block text-body font-medium">{{ d.number }}</span>
                  <span class="num text-xs text-text-secondary">{{ formatDate(d.date) }} · الإجمالي <MoneyText :value="d.total" plain /></span>
                </span>
                <span class="text-end">
                  <span class="block text-tiny text-text-secondary">المتبقي</span>
                  <MoneyText :value="d.outstanding" class="font-medium text-warning" />
                </span>
              </label>
            </li>
          </ul>
        </AppCard>
      </div>

      <AppCard title="تفاصيل السند" padding="sm">
        <div class="space-y-4">
          <AppInput v-model="amount" type="number" label="المبلغ" min="0" required :hint="selectedDoc ? `الحد الأقصى ${selectedDoc.outstanding.toFixed(2)}` : undefined" />
          <div>
            <span class="field-label">طريقة الدفع</span>
            <SegmentedControl v-model="method" :options="methodOptions" size="sm" />
          </div>
          <AppInput v-model="date" type="date" label="التاريخ" />
          <AppInput v-model="note" label="ملاحظات" />
          <ul v-if="submitted && problems.length" class="list-inside list-disc text-xs text-danger">
            <li v-for="p in problems" :key="p">{{ p }}</li>
          </ul>
          <AppButton variant="primary" block :icon="Save" :loading="saving" @click="save">حفظ السند</AppButton>
          <p class="text-tiny leading-5 text-text-secondary">
            {{ isReceived ? 'القيد: من الصندوق/البنك إلى حساب العملاء.' : 'القيد: من حساب الموردين إلى الصندوق/البنك.' }}
          </p>
        </div>
      </AppCard>
    </div>
  </div>
</template>
