<script setup lang="ts">
/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §2 "Full invoice form (desk)"). Used both at
 * `/sales/invoices/new` (posts a real `SalesInvoice`, `source: 'DESK'`) and `/sales/quotations/new`
 * (`asQuotation`, never posts — saves a `Quotation` instead). Shares `totals.ts` with the POS.
 */
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { FileText, Plus, Save, Send } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import FormActions from '@/modules/core/components/blocks/FormActions.vue';
import FormField from '@/modules/core/components/blocks/FormField.vue';
import FormPage from '@/modules/core/components/layouts/FormPage.vue';
import FormSection from '@/modules/core/components/blocks/FormSection.vue';
import JournalPreview from '@/modules/core/components/JournalPreview.vue';
import type { LineColumn } from '@/modules/core/components/blocks/LineItemsEditor.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import TotalsPanel, { type TotalsRow } from '@/modules/core/components/blocks/TotalsPanel.vue';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { getAccounts } from '@/modules/accounting/services/accountingService';
import { computeDueDate } from '@/modules/parties/helpers/creditLimit';
import { getCustomers } from '@/modules/parties/services/partyService';
import type { Customer } from '@/modules/parties/types';
import { useCatalogStore } from '@/modules/products/controllers/useCatalogStore';
import { getProducts } from '@/modules/products/services/productService';
import type { Product } from '@/modules/products/types';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import InvoiceLinesGrid, { type DeskLine } from '../components/InvoiceLinesGrid.vue';
import TenderDialog from '../components/TenderDialog.vue';
import { computeInvoiceTotals } from '../helpers/totals';
import { createSale, previewSale, saveQuotation, setQuotationStatus } from '../services/invoiceService';
import type { JournalPreviewLine, SaleInput, Tender } from '../types';

const props = defineProps<{ asQuotation?: boolean }>();
const router = useRouter();
const toast = useToast();
const settings = useSettingsStore();
const catalog = useCatalogStore();

const products = ref<Product[]>([]);
const customers = ref<Customer[]>([]);
const revenueAccounts = ref<{ id: string; code: string; name: string }[]>([]);
const loading = ref(true);
const saving = ref(false);
const ownerRef = ref(`draft-invoice-${Date.now()}`);

const customerId = ref<string | undefined>();
const invoiceDate = ref(new Date().toISOString().slice(0, 10));
const dueDate = ref('');
const invoiceType = ref<'STANDARD' | 'SIMPLIFIED'>('SIMPLIFIED');
const invoiceTypeTouched = ref(false);
const poReference = ref('');
const note = ref('');
const terms = ref('');
const expiryDate = ref('');
const lines = ref<DeskLine[]>([]);
const discountRate = ref(0);
const tenderOpen = ref(false);
const payNow = ref(false);
const journalPreview = ref<JournalPreviewLine[]>([]);
const journalError = ref('');
const showJournal = ref(false);

function newLine(): DeskLine {
  return { id: `dl-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`, name: '', qty: 1, price: 0, discount: 0, discountIsPct: true, isFreeText: false };
}

async function load() {
  loading.value = true;
  try {
    const [p, c, accounts] = await Promise.all([getProducts(), getCustomers(), getAccounts(), catalog.load()]);
    products.value = p;
    customers.value = c;
    revenueAccounts.value = accounts.filter((a) => !a.isGroup && a.kind === 'REVENUE').map((a) => ({ id: a.id, code: a.code, name: a.name }));
    if (!lines.value.length) lines.value = [newLine()];
  } catch (err) {
    toast.error(err);
  } finally {
    loading.value = false;
  }
}
onMounted(load);

const customer = computed(() => customers.value.find((c) => c.id === customerId.value));
watch(customer, (c) => {
  if (!invoiceTypeTouched.value) invoiceType.value = c?.vatNumber ? 'STANDARD' : 'SIMPLIFIED';
  if (c && !dueDate.value) {
    const computed = computeDueDate(new Date(invoiceDate.value).toISOString(), c.paymentTermsDays);
    if (computed) dueDate.value = computed.slice(0, 10);
  }
});

const customerOptions = computed(() => customers.value.map((c) => ({ value: c.id, label: c.name, sublabel: c.vatNumber ? `ض.ق.م ${c.vatNumber}` : c.phone })));

function taxOf(taxId: string | undefined) {
  const tax = (taxId && settings.taxes.find((t) => t.id === taxId && t.active)) ?? settings.taxes.find((t) => t.id === settings.settings?.defaultTaxId && t.active);
  return tax ? { rate: tax.rate, category: tax.category } : { rate: settings.salesTaxRate, category: 'S' as const };
}

const totals = computed(() =>
  computeInvoiceTotals(
    lines.value.filter((l) => l.qty > 0).map((l) => ({ qty: l.qty, unitPrice: l.price, discount: l.discount, discountIsPct: l.discountIsPct, tax: taxOf(l.taxId) })),
    discountRate.value > 0 ? { pct: discountRate.value } : undefined,
    settings.settings?.pricesIncludeTax !== false,
  ),
);

function addLine() {
  lines.value.push(newLine());
}
function duplicateLine(i: number) {
  lines.value.splice(i + 1, 0, { ...lines.value[i], id: `dl-${Date.now()}-${Math.random().toString(36).slice(2, 7)}` });
}
function removeLine(i: number) {
  lines.value.splice(i, 1);
  if (!lines.value.length) lines.value = [newLine()];
}
function onLinesChange(next: DeskLine[]) {
  lines.value = next;
}

const draft = computed<Omit<SaleInput, 'paymentMethod' | 'paidAmount' | 'tenderedAmount' | 'tenders'>>(() => ({
  customerId: customerId.value,
  discountRate: discountRate.value,
  note: note.value || undefined,
  lines: lines.value
    .filter((l) => l.qty > 0 && (l.productId || l.isFreeText))
    .map((l) => ({
      productId: l.productId ?? `freetext`,
      qty: l.qty,
      price: l.price,
      discount: l.discount || undefined,
      discountIsPct: l.discountIsPct,
      taxId: l.taxId,
      isFreeText: l.isFreeText || !l.productId,
      revenueAccountId: l.isFreeText ? l.revenueAccountId : undefined,
      name: l.name,
    })),
  source: 'DESK',
  invoiceType: invoiceType.value,
  dueDateOverride: dueDate.value ? new Date(dueDate.value).toISOString() : undefined,
  poReference: poReference.value || undefined,
  terms: terms.value || undefined,
}));

let previewTimer: ReturnType<typeof setTimeout> | undefined;
watch(
  draft,
  () => {
    clearTimeout(previewTimer);
    if (!draft.value.lines.length) return;
    previewTimer = setTimeout(async () => {
      try {
        journalPreview.value = await previewSale({ ...draft.value, paymentMethod: 'credit', paidAmount: 0 });
        journalError.value = '';
      } catch (err) {
        journalError.value = errorMessage(err);
      }
    }, 250);
  },
  { deep: true },
);

const validLines = computed(() => draft.value.lines.length > 0);
const freeTextMissingAccount = computed(() => lines.value.some((l) => l.isFreeText && l.qty > 0 && !l.revenueAccountId));

async function saveAsQuotation(status: 'DRAFT' | 'SENT') {
  if (!validLines.value) {
    toast.warning('أضف صنفاً واحداً على الأقل');
    return;
  }
  saving.value = true;
  try {
    const q = await saveQuotation({
      customerId: customerId.value,
      expiryDate: expiryDate.value ? new Date(expiryDate.value).toISOString() : undefined,
      lines: draft.value.lines,
      discountRate: discountRate.value,
      note: note.value || undefined,
      terms: terms.value || undefined,
      poReference: poReference.value || undefined,
    });
    if (status === 'SENT') await setQuotationStatus(q.id, 'SENT');
    toast.success('تم حفظ عرض السعر', q.number);
    router.push({ name: 'quotation', params: { id: q.id } });
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

function postInvoice() {
  if (!validLines.value) {
    toast.warning('أضف صنفاً واحداً على الأقل');
    return;
  }
  if (freeTextMissingAccount.value) {
    toast.warning('اختر حساب الإيراد للسطر النصي الحر');
    return;
  }
  if (totals.value.gross > 0) {
    payNow.value = false;
    tenderOpen.value = true;
  } else {
    void finish({ tenders: [], paidAmount: 0 });
  }
}

async function finish(payment: { tenders: Tender[]; paidAmount: number; tenderedAmount?: number }, andPrint = false) {
  saving.value = true;
  try {
    const invoice = await createSale({ ...draft.value, paymentMethod: 'cash', ...payment });
    tenderOpen.value = false;
    toast.success('تم حفظ الفاتورة', invoice.number);
    router.push(andPrint ? { name: 'invoice-print', params: { id: invoice.id }, query: { auto: '1' } } : { name: 'invoice', params: { id: invoice.id } });
  } catch (err) {
    toast.error(err, 'تعذر حفظ الفاتورة');
  } finally {
    saving.value = false;
  }
}

// --- LineItemsEditor wiring (render-only — see InvoiceLinesGrid.vue) ---------------------------
const lineColumns: LineColumn<DeskLine>[] = [
  { key: 'name', label: 'الصنف / الوصف', type: 'custom' },
  { key: 'qty', label: 'الكمية', type: 'custom', width: '80px' },
  { key: 'price', label: 'السعر', type: 'custom', width: '96px' },
  { key: 'discount', label: 'خصم%', type: 'custom', width: '80px' },
  { key: 'net', label: 'الصافي', type: 'custom', width: '96px' },
  { key: 'vat', label: 'الضريبة', type: 'custom', width: '96px' },
  { key: 'gross', label: 'الإجمالي', type: 'custom', width: '96px' },
];

const totalsRows = computed<TotalsRow[]>(() => [
  { label: 'الصافي', amount: totals.value.net },
  ...(totals.value.invoiceDiscountAmount > 0 ? [{ label: 'الخصم', amount: totals.value.invoiceDiscountAmount, negative: true }] : []),
  ...totals.value.vatByCategory.map((g) => ({ label: `ضريبة ${g.category} (${g.rate}%)`, amount: g.vat })),
  { label: 'الإجمالي', amount: totals.value.gross, emphasis: true },
]);
</script>

<template>
  <FormPage
    :title="asQuotation ? 'عرض سعر جديد' : 'فاتورة مبيعات جديدة'"
    :subtitle="asQuotation ? 'يمكن تحويله لفاتورة لاحقاً' : 'فاتورة ضريبية كاملة'"
    :back="{ name: 'invoices' }"
  >
    <FormSection title="بيانات الفاتورة" :columns="2">
      <FormField label="العميل">
        <AppCombobox v-model="customerId" :options="customerOptions" placeholder="عميل نقدي" search-placeholder="اسم أو رقم ضريبي" clearable />
      </FormField>
      <div class="grid grid-cols-2 gap-4 sm:grid-cols-3">
        <AppDatePicker v-model="invoiceDate" label="التاريخ" />
        <AppDatePicker v-model="dueDate" label="تاريخ الاستحقاق" />
        <AppDatePicker v-if="asQuotation" v-model="expiryDate" label="صالح حتى" />
      </div>
      <FormField label="نوع الفاتورة">
        <AppSelect
          v-model="invoiceType"
          :options="[
            { value: 'STANDARD', label: 'فاتورة ضريبية' },
            { value: 'SIMPLIFIED', label: 'فاتورة ضريبية مبسطة' },
          ]"
          @update:model-value="invoiceTypeTouched = true"
        />
      </FormField>
      <FormField label="مرجع أمر الشراء">
        <AppInput v-model="poReference" />
      </FormField>
      <div v-if="customer" class="col-span-full rounded-lg bg-surface p-3 text-body">
        <p v-if="customer.vatNumber">الرقم الضريبي: <span class="num">{{ customer.vatNumber }}</span></p>
        <p v-if="customer.address">{{ customer.address }}</p>
        <p>الرصيد الحالي: <MoneyText :value="customer.balance" plain /> <span v-if="customer.creditLimit">/ الحد الائتماني <MoneyText :value="customer.creditLimit" plain /></span></p>
      </div>
    </FormSection>

    <FormSection title="الأصناف">
      <div class="-mt-2 mb-2 flex justify-end">
        <AppButton size="sm" variant="ghost" :icon="Plus" @click="addLine">إضافة سطر (Ctrl+Enter)</AppButton>
      </div>
      <InvoiceLinesGrid
        :lines="lines"
        :columns="lineColumns"
        :new-line="newLine"
        :products="products"
        :revenue-accounts="revenueAccounts"
        :line-totals="totals.lines"
        @lines-change="onLinesChange"
        @add-line="addLine"
        @duplicate-line="duplicateLine"
        @remove-line="removeLine"
      />
    </FormSection>

    <FormSection title="ملاحظات وشروط" :columns="2">
      <FormField label="ملاحظات">
        <AppTextarea v-model="note" :rows="3" />
      </FormField>
      <FormField label="الشروط والأحكام">
        <AppTextarea v-model="terms" :rows="3" placeholder="مثال: السداد خلال 30 يوماً من تاريخ الفاتورة" />
      </FormField>
    </FormSection>

    <FormSection title="المرفقات">
      <AttachmentField :owner-ref="ownerRef" />
    </FormSection>

    <template #aside>
      <TotalsPanel :rows="totalsRows" :currency="settings.currency" show-tafqit :tafqit-amount="totals.gross" />
      <FormField label="خصم الفاتورة %">
        <AppInput v-model.number="discountRate" type="number" min="0" max="100" />
      </FormField>

      <div class="rounded-xl border border-border bg-surface p-4">
        <button type="button" class="text-xs text-text-secondary hover:text-text-primary" @click="showJournal = !showJournal">
          {{ showJournal ? '▾' : '◂' }} معاينة القيد المحاسبي
        </button>
        <div v-if="showJournal" class="mt-2">
          <p v-if="journalError" class="text-xs text-danger">{{ journalError }}</p>
          <JournalPreview v-else :lines="journalPreview" />
        </div>
      </div>
    </template>

    <template #actions>
      <FormActions>
        <template v-if="asQuotation" #primary>
          <AppButton variant="primary" :icon="Send" :loading="saving" @click="saveAsQuotation('SENT')">حفظ وإرسال</AppButton>
          <AppButton :icon="Save" :loading="saving" @click="saveAsQuotation('DRAFT')">حفظ كمسودة</AppButton>
        </template>
        <template v-else #primary>
          <AppButton variant="primary" :icon="FileText" :loading="saving" @click="postInvoice">ترحيل الفاتورة</AppButton>
          <AppButton v-if="totals.gross <= 0" :loading="saving" @click="finish({ tenders: [], paidAmount: 0 }, true)">ترحيل وطباعة</AppButton>
        </template>
      </FormActions>
    </template>

    <TenderDialog v-model:open="tenderOpen" :total="totals.gross" :customer-name="customer?.name" :has-customer="!!customerId" :draft="draft" :submitting="saving" @confirm="(p) => finish(p)" />
  </FormPage>
</template>
