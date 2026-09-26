<script setup lang="ts">
/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §2 "Full invoice form (desk)"). Used both at
 * `/sales/invoices/new` (posts a real `SalesInvoice`, `source: 'DESK'`) and `/sales/quotations/new`
 * (`asQuotation`, never posts — saves a `Quotation` instead). Shares `totals.ts` with the POS.
 */
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { FileText, Plus, Save, Send, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import JournalPreview from '@/modules/core/components/JournalPreview.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { useGridTab } from '@/modules/core/controllers/useGridTab';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { tafqit } from '@/modules/core/helpers/tafqit';
import { getAccounts } from '@/modules/accounting/services/accountingService';
import { computeDueDate } from '@/modules/parties/helpers/creditLimit';
import { getCustomers } from '@/modules/parties/services/partyService';
import type { Customer } from '@/modules/parties/types';
import { useCatalogStore } from '@/modules/products/controllers/useCatalogStore';
import { getProducts } from '@/modules/products/services/productService';
import type { Product } from '@/modules/products/types';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import TenderDialog from '../components/TenderDialog.vue';
import { computeInvoiceTotals } from '../helpers/totals';
import { createSale, previewSale, saveQuotation, setQuotationStatus } from '../services/invoiceService';
import type { JournalPreviewLine, SaleInput, Tender } from '../types';

const props = defineProps<{ asQuotation?: boolean }>();
const router = useRouter();
const toast = useToast();
const settings = useSettingsStore();
const catalog = useCatalogStore();

interface DeskLine {
  id: string;
  productId?: string;
  name: string;
  unitId?: string;
  qty: number;
  price: number;
  discount: number;
  discountIsPct: boolean;
  taxId?: string;
  isFreeText: boolean;
  revenueAccountId?: string;
}

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

function pickProduct(line: DeskLine, product: Product) {
  line.productId = product.id;
  line.name = product.name;
  line.price = product.price;
  line.taxId = product.saleTaxId;
  line.isFreeText = false;
}

function addLine() {
  lines.value.push(newLine());
}
const linesBody = ref<HTMLElement>();
const onLinesKeydown = useGridTab({ container: linesBody, addRow: addLine, isFilled: (i) => !!lines.value[i]?.name.trim() });
function duplicateLine(i: number) {
  lines.value.splice(i + 1, 0, { ...lines.value[i], id: `dl-${Date.now()}-${Math.random().toString(36).slice(2, 7)}` });
}
function removeLine(i: number) {
  lines.value.splice(i, 1);
  if (!lines.value.length) lines.value = [newLine()];
}
function toggleFreeText(line: DeskLine) {
  line.isFreeText = !line.isFreeText;
  if (line.isFreeText) line.productId = undefined;
}

/**
 * `<datalist>` options can't carry a click handler (native browser UI, not scriptable) — the only
 * observable signal when a suggestion is chosen is the `<input>`'s value changing to match an
 * option exactly. So resolve the product on every `name` change instead of relying on `pickProduct`
 * being called from the option itself: an exact (or normalized-Arabic) name match auto-attaches the
 * product id/price/tax; anything else is left for the cashier to mark "سطر حر" explicitly.
 */
function resolveLineByName(line: DeskLine) {
  const typed = line.name.trim();
  if (!typed) return;
  const match = products.value.find((p) => p.name === typed);
  if (match) {
    pickProduct(line, match);
  }
}

/** Paste from Excel (docs/v2/06 §2): tab-separated rows → name / qty / price / discount%. */
function onPaste(e: ClipboardEvent, startIndex: number) {
  const text = e.clipboardData?.getData('text/plain');
  if (!text?.includes('\t') && !text?.includes('\n')) return;
  e.preventDefault();
  const rows = text
    .split(/\r?\n/)
    .filter((r) => r.trim())
    .map((r) => r.split('\t'));
  rows.forEach((cells, i) => {
    const idx = startIndex + i;
    if (!lines.value[idx]) lines.value.push(newLine());
    const l = lines.value[idx];
    l.name = cells[0]?.trim() ?? l.name;
    l.qty = num0(cells[1]) || l.qty || 1;
    l.price = num0(cells[2]) || l.price;
    l.discount = num0(cells[3]) || 0;
    l.isFreeText = true;
  });
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

const amountInWords = computed(() => tafqit(totals.value.gross));
</script>

<template>
  <div>
    <PageHeader :title="asQuotation ? 'عرض سعر جديد' : 'فاتورة مبيعات جديدة'" :subtitle="asQuotation ? 'يمكن تحويله لفاتورة لاحقاً' : 'فاتورة ضريبية كاملة'" :back="{ name: 'invoices' }" />

    <div class="grid items-start gap-5 xl:grid-cols-[1fr_340px]">
      <div class="space-y-5">
        <AppCard title="بيانات الفاتورة">
          <div class="grid grid-cols-2 gap-4 md:grid-cols-4">
            <AppCombobox v-model="customerId" class="col-span-2" :options="customerOptions" placeholder="عميل نقدي" search-placeholder="اسم أو رقم ضريبي" clearable />
            <AppDatePicker v-model="invoiceDate" label="التاريخ" />
            <AppDatePicker v-model="dueDate" label="تاريخ الاستحقاق" />
            <AppSelect
              v-model="invoiceType"
              label="نوع الفاتورة"
              :options="[
                { value: 'STANDARD', label: 'فاتورة ضريبية' },
                { value: 'SIMPLIFIED', label: 'فاتورة ضريبية مبسطة' },
              ]"
              @update:model-value="invoiceTypeTouched = true"
            />
            <div>
              <label class="field-label" for="po-ref">مرجع أمر الشراء</label>
              <input id="po-ref" v-model="poReference" class="control h-10" />
            </div>
            <AppDatePicker v-if="asQuotation" v-model="expiryDate" label="صالح حتى" />
          </div>
          <div v-if="customer" class="mt-3 rounded-lg bg-surface p-3 text-body">
            <p v-if="customer.vatNumber">الرقم الضريبي: <span class="num">{{ customer.vatNumber }}</span></p>
            <p v-if="customer.address">{{ customer.address }}</p>
            <p>الرصيد الحالي: <MoneyText :value="customer.balance" plain /> <span v-if="customer.creditLimit">/ الحد الائتماني <MoneyText :value="customer.creditLimit" plain /></span></p>
          </div>
        </AppCard>

        <AppCard title="الأصناف" padding="none">
          <template #actions>
            <AppButton size="sm" variant="ghost" :icon="Plus" @click="addLine">إضافة سطر (Ctrl+Enter)</AppButton>
          </template>
          <table class="w-full text-body">
            <thead class="bg-surface text-xs text-text-secondary">
              <tr class="border-b border-border">
                <th class="px-3 py-2 text-start font-medium">الصنف / الوصف</th>
                <th class="w-20 px-2 py-2 text-start font-medium">الكمية</th>
                <th class="w-24 px-2 py-2 text-start font-medium">السعر</th>
                <th class="w-20 px-2 py-2 text-start font-medium">خصم%</th>
                <th class="w-24 px-2 py-2 text-start font-medium">الصافي</th>
                <th class="w-24 px-2 py-2 text-start font-medium">الضريبة</th>
                <th class="w-24 px-2 py-2 text-start font-medium">الإجمالي</th>
                <th class="w-16 px-2 py-2" />
              </tr>
            </thead>
            <tbody ref="linesBody" @keydown="onLinesKeydown">
              <tr v-for="(l, i) in lines" :key="l.id" class="border-b border-border last:border-0">
                <td class="px-3 py-1.5">
                  <input
                    v-model="l.name"
                    class="control h-9 w-full"
                    :placeholder="l.isFreeText ? 'وصف حر' : 'ابحث عن منتج أو اكتب وصفاً'"
                    :list="`prod-list-${i}`"
                    @paste="(e) => onPaste(e, i)"
                    @change="resolveLineByName(l)"
                    @keydown.ctrl.enter.prevent="addLine"
                    @keydown.ctrl.d.prevent="duplicateLine(i)"
                    @keydown.ctrl.delete.prevent="removeLine(i)"
                  />
                  <datalist :id="`prod-list-${i}`">
                    <option v-for="p in products" :key="p.id" :value="p.name" @click="pickProduct(l, p)" />
                  </datalist>
                  <button type="button" class="mt-1 text-tiny text-text-secondary hover:text-primary" @click="toggleFreeText(l)">
                    {{ l.isFreeText ? 'سطر حر ✓' : 'سطر حر؟' }}
                  </button>
                  <AppSelect
                    v-if="l.isFreeText"
                    v-model="l.revenueAccountId"
                    class="mt-1"
                    :options="revenueAccounts.map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` }))"
                    placeholder="حساب الإيراد (مطلوب)"
                  />
                </td>
                <td class="px-2 py-1.5"><input v-model.number="l.qty" type="number" min="0" step="0.001" class="control h-9 w-full" /></td>
                <td class="px-2 py-1.5"><input v-model.number="l.price" type="number" min="0" step="0.01" class="control h-9 w-full" /></td>
                <td class="px-2 py-1.5"><input v-model.number="l.discount" type="number" min="0" class="control h-9 w-full" /></td>
                <td class="px-2 py-1.5"><span class="num text-text-secondary">{{ formatNumber(totals.lines[i]?.net ?? 0) }}</span></td>
                <td class="px-2 py-1.5"><span class="num text-text-secondary">{{ formatNumber(totals.lines[i]?.vat ?? 0) }}</span></td>
                <td class="px-2 py-1.5"><span class="num font-medium">{{ formatNumber(totals.lines[i]?.gross ?? 0) }}</span></td>
                <td class="px-2 py-1.5 text-center">
                  <button type="button" class="rounded p-1 text-text-secondary hover:bg-danger/10 hover:text-danger" aria-label="حذف" data-grid-skip @click="removeLine(i)">
                    <Trash class="size-3.5" />
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </AppCard>

        <AppCard title="ملاحظات وشروط">
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="field-label" for="notes">ملاحظات</label>
              <textarea id="notes" v-model="note" class="control h-20 resize-none" />
            </div>
            <div>
              <label class="field-label" for="terms">الشروط والأحكام</label>
              <textarea id="terms" v-model="terms" class="control h-20 resize-none" placeholder="مثال: السداد خلال 30 يوماً من تاريخ الفاتورة" />
            </div>
          </div>
        </AppCard>

        <AppCard title="المرفقات">
          <AttachmentField :owner-ref="ownerRef" />
        </AppCard>
      </div>

      <div class="space-y-4">
        <AppCard title="الإجمالي" padding="sm">
          <div class="mb-3">
            <label class="field-label" for="inv-discount">خصم الفاتورة %</label>
            <input id="inv-discount" v-model.number="discountRate" type="number" min="0" max="100" class="control h-9" />
          </div>
          <dl class="space-y-1.5 text-body">
            <div class="flex justify-between"><dt class="text-text-secondary">الصافي</dt><dd><MoneyText :value="totals.net" /></dd></div>
            <div v-if="totals.invoiceDiscountAmount > 0" class="flex justify-between text-danger"><dt>الخصم</dt><dd>−<MoneyText :value="totals.invoiceDiscountAmount" plain /></dd></div>
            <div v-for="g in totals.vatByCategory" :key="`${g.category}-${g.rate}`" class="flex justify-between text-text-secondary">
              <dt>ضريبة {{ g.category }} ({{ g.rate }}%)</dt><dd><MoneyText :value="g.vat" plain /></dd>
            </div>
            <div class="flex items-baseline justify-between border-t border-border pt-2 text-lg font-semibold"><dt>الإجمالي</dt><dd><MoneyText :value="totals.gross" /></dd></div>
          </dl>
          <p class="mt-3 text-tiny leading-5 text-text-secondary">{{ amountInWords }}</p>
        </AppCard>

        <AppCard padding="sm">
          <button type="button" class="text-xs text-text-secondary hover:text-text-primary" @click="showJournal = !showJournal">
            {{ showJournal ? '▾' : '◂' }} معاينة القيد المحاسبي
          </button>
          <div v-if="showJournal" class="mt-2">
            <p v-if="journalError" class="text-xs text-danger">{{ journalError }}</p>
            <JournalPreview v-else :lines="journalPreview" />
          </div>
        </AppCard>

        <div class="space-y-2">
          <template v-if="asQuotation">
            <AppButton block variant="primary" :icon="Send" :loading="saving" @click="saveAsQuotation('SENT')">حفظ وإرسال</AppButton>
            <AppButton block :icon="Save" :loading="saving" @click="saveAsQuotation('DRAFT')">حفظ كمسودة</AppButton>
          </template>
          <template v-else>
            <AppButton block variant="primary" :icon="FileText" :loading="saving" @click="postInvoice">ترحيل الفاتورة</AppButton>
            <AppButton v-if="totals.gross <= 0" block :loading="saving" @click="finish({ tenders: [], paidAmount: 0 }, true)">ترحيل وطباعة</AppButton>
          </template>
        </div>
      </div>
    </div>

    <TenderDialog v-model:open="tenderOpen" :total="totals.gross" :customer-name="customer?.name" :has-customer="!!customerId" :draft="draft" :submitting="saving" @confirm="(p) => finish(p)" />
  </div>
</template>
