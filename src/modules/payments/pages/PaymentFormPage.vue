<script setup lang="ts">
// v2 §5 (docs/v2/14-platform.md §5 "Accounting and payments: ... payments"): a receipt/scan can be
// attached to the payment while filling the form, same draft-owner-ref pattern as ExpenseFormPage.
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Banknote, CircleCheck, CreditCard, Landmark, Save, Wand2, X } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, formatDate, todayKey } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { round2 } from '@/modules/invoices/helpers/totals';
import { getCustomers, getSuppliers } from '@/modules/parties/services/partyService';
import type { Customer, Supplier } from '@/modules/parties/types';
import { createPayment, getOpenDocuments } from '../services/paymentService';
import type { OpenDocument, PaymentAllocationInput, PaymentMethod, PaymentType } from '../types';

const route = useRoute();
const router = useRouter();
const toast = useToast();

const type = ref<PaymentType>(route.query.type === 'PAID' ? 'PAID' : 'RECEIVED');
const partyId = ref<string | undefined>(typeof route.query.party === 'string' ? route.query.party : undefined);
const preselectRef = typeof route.query.ref === 'string' ? route.query.ref : undefined;
const amount = ref<number | undefined>();
const method = ref<PaymentMethod>('cash');
const date = ref(todayKey());
const note = ref('');
const saving = ref(false);
const submitted = ref(false);

const draftOwnerRef = `payment:new:${Date.now()}`;

const customers = ref<Customer[]>([]);
const suppliers = ref<Supplier[]>([]);
const docs = ref<OpenDocument[]>([]);
const docsLoading = ref(false);
/** docId → allocated amount for this draft. */
const allocations = reactive<Record<string, number | undefined>>({});

const isReceived = computed(() => type.value === 'RECEIVED');

onMounted(async () => {
  [customers.value, suppliers.value] = await Promise.all([getCustomers(), getSuppliers()]);
  if (partyId.value) await loadDocs();
});

const partyOptions = computed(() =>
  (isReceived.value ? customers.value : suppliers.value)
    .slice()
    .sort((a, b) => b.balance - a.balance)
    .map((p) => ({ value: p.id, label: p.name, sublabel: p.balance > 0 ? `الرصيد ${p.balance.toFixed(2)}` : 'لا يوجد رصيد' })),
);

async function loadDocs() {
  docs.value = [];
  for (const k of Object.keys(allocations)) delete allocations[k];
  if (!partyId.value) return;
  docsLoading.value = true;
  try {
    docs.value = await getOpenDocuments(isReceived.value ? 'customer' : 'supplier', partyId.value);
    if (preselectRef && docs.value.some((d) => d.id === preselectRef)) {
      const doc = docs.value.find((d) => d.id === preselectRef)!;
      allocations[doc.id] = doc.outstanding;
      amount.value = doc.outstanding;
    }
  } finally {
    docsLoading.value = false;
  }
}

watch(type, () => {
  partyId.value = undefined;
  docs.value = [];
  for (const k of Object.keys(allocations)) delete allocations[k];
  method.value = type.value === 'PAID' ? 'bank_transfer' : 'cash';
});
watch(partyId, () => loadDocs());

const allocatedTotal = computed(() => Object.values(allocations).reduce((a: number, v) => a + num0(v), 0));
const unallocated = computed(() => Math.max(0, round2(num0(amount.value) - allocatedTotal.value)));
const overAllocated = computed(() => allocatedTotal.value > num0(amount.value) + 0.005);

function toggleDoc(doc: OpenDocument, checked: boolean) {
  if (checked) {
    const remaining = Math.max(0, round2(num0(amount.value) - allocatedTotal.value));
    allocations[doc.id] = Math.min(doc.outstanding, remaining || doc.outstanding);
  } else {
    delete allocations[doc.id];
  }
}

/** "تخصيص تلقائي" — oldest first, up to the payment amount (docs/v2/09-purchases-payments-expenses.md §3). */
function autoAllocate() {
  for (const k of Object.keys(allocations)) delete allocations[k];
  let remaining = num0(amount.value);
  for (const doc of docs.value) {
    if (remaining <= 0.005) break;
    const take = Math.min(doc.outstanding, remaining);
    allocations[doc.id] = round2(take);
    remaining = round2(remaining - take);
  }
}

function clearAllocations() {
  for (const k of Object.keys(allocations)) delete allocations[k];
}

const problems = computed(() => {
  const list: string[] = [];
  if (!partyId.value) list.push(isReceived.value ? 'اختر العميل' : 'اختر المورد');
  if (!(num0(amount.value) > 0)) list.push('أدخل مبلغاً أكبر من صفر');
  if (overAllocated.value) list.push('إجمالي التخصيص أكبر من مبلغ السند');
  return list;
});

async function save() {
  submitted.value = true;
  if (problems.value.length) return;
  saving.value = true;
  try {
    const allocationInputs: PaymentAllocationInput[] = docs.value
      .filter((d) => num0(allocations[d.id]) > 0)
      .map((d) => ({ targetKind: d.kind, targetId: d.id, amount: round2(num0(allocations[d.id])) }));
    const payment = await createPayment({
      date: dateKeyToIso(date.value),
      type: type.value,
      targetType: isReceived.value ? 'customer' : 'supplier',
      targetId: partyId.value!,
      amount: num0(amount.value),
      method: method.value,
      note: note.value.trim() || undefined,
      allocations: allocationInputs,
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
    <PageHeader :title="isReceived ? 'سند قبض' : 'سند صرف'" :subtitle="isReceived ? 'تحصيل مبلغ من عميل وتخصيصه على فاتورة واحدة أو أكثر' : 'سداد مبلغ لمورد وتخصيصه على أمر شراء واحد أو أكثر'" back="/payments" />

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

        <AppCard :title="`شبكة التخصيص — ${isReceived ? 'الفواتير المفتوحة' : 'أوامر الشراء غير المسددة'}`" padding="none">
          <template v-if="docs.length" #actions>
            <AppButton size="sm" :icon="Wand2" @click="autoAllocate">تخصيص تلقائي</AppButton>
            <AppButton size="sm" :icon="X" @click="clearAllocations">مسح</AppButton>
          </template>
          <div v-if="docsLoading" class="p-4"><SkeletonBlock :lines="3" height="h-10" /></div>
          <EmptyState
            v-else-if="!partyId"
            compact
            :title="isReceived ? 'اختر عميلاً لعرض فواتيره المفتوحة' : 'اختر مورداً لعرض أوامره غير المسددة'"
          />
          <EmptyState v-else-if="!docs.length" compact :icon="CircleCheck" title="لا توجد مستندات مفتوحة" description="كل المستندات مسددة بالكامل — يمكنك حفظ السند برصيد غير مخصص" />
          <table v-else class="w-full text-body">
            <thead class="border-b border-border text-xs text-text-secondary">
              <tr>
                <th class="w-8 px-3 py-2"></th>
                <th class="px-2 py-2 text-start">المستند</th>
                <th class="px-2 py-2 text-start">التاريخ</th>
                <th class="px-2 py-2 text-start">الاستحقاق</th>
                <th class="px-2 py-2 text-end">الإجمالي</th>
                <th class="px-2 py-2 text-end">المتبقي</th>
                <th class="w-32 px-3 py-2 text-end">تخصيص</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-border">
              <tr v-for="d in docs" :key="d.id">
                <td class="px-3 py-2">
                  <input
                    type="checkbox"
                    class="size-4 accent-[var(--color-primary)]"
                    :checked="allocations[d.id] !== undefined"
                    @change="toggleDoc(d, ($event.target as HTMLInputElement).checked)"
                  />
                </td>
                <td class="num px-2 py-2 font-medium">{{ d.number }}</td>
                <td class="num px-2 py-2 text-text-secondary">{{ formatDate(d.date) }}</td>
                <td class="num px-2 py-2 text-text-secondary">{{ d.dueDate ? formatDate(d.dueDate) : '—' }}</td>
                <td class="px-2 py-2 text-end"><MoneyText :value="d.total" plain /></td>
                <td class="px-2 py-2 text-end"><MoneyText :value="d.outstanding" plain class="font-medium text-warning" /></td>
                <td class="px-3 py-2">
                  <input
                    type="number"
                    min="0"
                    :max="d.outstanding"
                    class="control num h-8 w-full text-end"
                    :value="allocations[d.id] ?? ''"
                    :disabled="allocations[d.id] === undefined"
                    @input="allocations[d.id] = num0(($event.target as HTMLInputElement).value)"
                  />
                </td>
              </tr>
            </tbody>
          </table>
          <div v-if="docs.length" class="flex items-center justify-between border-t border-border px-4 py-2.5 text-xs">
            <span class="text-text-secondary">مخصص <span class="num font-medium text-text-primary">{{ allocatedTotal.toFixed(2) }}</span></span>
            <span :class="unallocated > 0.005 ? 'text-warning' : 'text-text-secondary'">
              غير مخصص (يبقى رصيداً للطرف) <span class="num font-medium">{{ unallocated.toFixed(2) }}</span>
            </span>
          </div>
        </AppCard>
      </div>

      <AppCard title="تفاصيل السند" padding="sm">
        <div class="space-y-4">
          <AppInput v-model="amount" type="number" label="المبلغ" min="0" required />
          <div>
            <span class="field-label">طريقة الدفع</span>
            <SegmentedControl v-model="method" :options="methodOptions" size="sm" />
          </div>
          <AppDatePicker v-model="date" label="التاريخ" />
          <AppInput v-model="note" label="ملاحظات" />
          <div>
            <span class="field-label">المرفقات</span>
            <AttachmentField :owner-ref="draftOwnerRef" />
          </div>
          <ul v-if="submitted && problems.length" class="list-inside list-disc text-xs text-danger">
            <li v-for="p in problems" :key="p">{{ p }}</li>
          </ul>
          <AppButton variant="primary" block :icon="Save" :loading="saving" @click="save">حفظ السند</AppButton>
          <p class="text-tiny leading-5 text-text-secondary">
            {{ isReceived ? 'القيد: من الصندوق/البنك إلى حساب العملاء — مرة واحدة، مهما بلغ عدد المستندات المخصصة.' : 'القيد: من حساب الموردين إلى الصندوق/البنك — مرة واحدة، مهما بلغ عدد المستندات المخصصة.' }}
          </p>
        </div>
      </AppCard>
    </div>
  </div>
</template>
