<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { AlertTriangle, PackageCheck, PackageX, Plus, Save, Send, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import FormActions from '@/modules/core/components/blocks/FormActions.vue';
import FormField from '@/modules/core/components/blocks/FormField.vue';
import FormPage from '@/modules/core/components/layouts/FormPage.vue';
import FormSection from '@/modules/core/components/blocks/FormSection.vue';
import type { LineColumn } from '@/modules/core/components/blocks/LineItemsEditor.vue';
import TotalsPanel, { type TotalsRow } from '@/modules/core/components/blocks/TotalsPanel.vue';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, formatNumber, toDateKey, todayKey } from '@/modules/core/helpers/format';
import { num0, toNum } from '@/modules/core/helpers/numbers';
import { round2 } from '@/modules/invoices/helpers/totals';
import { getSuppliers } from '@/modules/parties/services/partyService';
import type { Supplier } from '@/modules/parties/types';
import { getProducts, isLowStock } from '@/modules/products/services/productService';
import type { Product } from '@/modules/products/types';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { computePurchaseTotals } from '@/mocks/backend/purchases';
import PurchaseLinesGrid, { type PurchaseLine } from '../components/PurchaseLinesGrid.vue';
import { getPurchaseOrder, savePurchaseOrder, sendPurchaseOrderToSupplier } from '../services/purchaseService';
import type { LandedCostLineInput, LandedCostSpread } from '../types';

type Line = PurchaseLine;

interface LandedRow {
  key: number;
  label: string;
  amount?: number;
  supplierId?: string;
  spreadBy: LandedCostSpread;
}

const route = useRoute<'purchase-new' | 'purchase-edit'>();
const router = useRouter();
const toast = useToast();
const settings = useSettingsStore();
const id = computed(() => ('id' in route.params ? String(route.params.id) : undefined));

const supplierId = ref<string | undefined>(typeof route.query.supplier === 'string' ? route.query.supplier : undefined);
const date = ref(todayKey());
const note = ref('');
const supplierInvoiceNo = ref('');
const supplierInvoiceDate = ref('');
let seq = 0;
let lcSeq = 0;
const lines = ref<Line[]>([{ key: ++seq, discountIsPct: true }]);
const landedCosts = ref<LandedRow[]>([]);
const invoiceDiscountAmount = ref<number>();
const attachmentIds = ref<string[]>([]);
const products = ref<Product[]>([]);
const suppliers = ref<Supplier[]>([]);
const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref<'draft' | 'send' | 'receive' | null>(null);
const submitted = ref(false);
const posted = ref(false);
const draftOwnerRef = computed(() => `purchase:${id.value ?? 'new'}`);

const byId = computed(() => new Map(products.value.map((p) => [p.id, p])));

onMounted(async () => {
  try {
    const [p, s] = await Promise.all([getProducts(), getSuppliers()]);
    products.value = p;
    suppliers.value = s;
    if (id.value) {
      const po = await getPurchaseOrder(id.value);
      if (po.status !== 'DRAFT') {
        router.replace({ name: 'purchase', params: { id: po.id } });
        return;
      }
      supplierId.value = po.supplierId;
      date.value = toDateKey(po.date);
      note.value = po.note ?? '';
      supplierInvoiceNo.value = po.supplierInvoiceNo ?? '';
      supplierInvoiceDate.value = po.supplierInvoiceDate ? toDateKey(po.supplierInvoiceDate) : '';
      attachmentIds.value = po.attachmentIds ?? [];
      invoiceDiscountAmount.value = po.invoiceDiscount?.amount;
      lines.value = po.lines.map((l) => ({ key: ++seq, discountIsPct: true, ...l }));
      landedCosts.value = (po.landedCosts ?? []).map((l) => ({ key: ++lcSeq, ...l }));
    }
  } catch (err) {
    loadError.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
});

const supplierOptions = computed(() => suppliers.value.map((s) => ({ value: s.id, label: s.name, sublabel: s.contactPerson })));
const selectedSupplier = computed(() => suppliers.value.find((s) => s.id === supplierId.value));
const productOptions = computed(() =>
  products.value.map((p) => ({
    value: p.id,
    label: p.name,
    sublabel: `${p.sku} · المتوفر ${p.type === 'service' ? '—' : formatNumber(p.stockQty)}${isLowStock(p) ? ' · منخفض' : ''}`,
    keywords: `${p.sku} ${p.barcode ?? ''}`,
  })),
);
const taxOptions = computed(() => [{ value: '', label: 'ضريبة المشتريات الافتراضية' }, ...db_taxOptions()]);
function db_taxOptions() {
  return (settings.taxes ?? []).filter((t) => t.direction === 'purchase').map((t) => ({ value: t.id, label: `${t.name} (${t.rate}%)` }));
}

function unitOptionsFor(productId: string | undefined) {
  const p = productId ? byId.value.get(productId) : undefined;
  if (!p?.units?.length) return [];
  return p.units.map((u) => ({ value: u.id, label: `${u.factor}× (${u.defaultForPurchase ? 'شراء افتراضي' : 'وحدة'})` }));
}

function onProductSelected(line: Line) {
  const p = line.productId ? byId.value.get(line.productId) : undefined;
  if (!p) return;
  if (toNum(line.costPrice) === undefined) line.costPrice = p.costPrice;
  if (toNum(line.qty) === undefined) line.qty = Math.max(1, (p.minStock ?? 5) * 3 - Math.max(0, p.stockQty));
  const defaultUnit = p.units?.find((u) => u.defaultForPurchase) ?? p.units?.find((u) => u.factor === 1);
  line.unitId = defaultUnit?.id;
  if (defaultUnit && defaultUnit.factor !== 1) line.costPrice = defaultUnit.price;
  if (lines.value.every((l) => l.productId)) lines.value.push({ key: ++seq, discountIsPct: true });
}

function addLowStock() {
  const existing = new Set(lines.value.map((l) => l.productId));
  const low = products.value.filter((p) => isLowStock(p) && !existing.has(p.id));
  if (!low.length) {
    toast.info('لا توجد أصناف منخفضة المخزون');
    return;
  }
  lines.value = lines.value.filter((l) => l.productId);
  for (const p of low) lines.value.push({ key: ++seq, discountIsPct: true, productId: p.id, qty: Math.max(1, (p.minStock ?? 5) * 3 - Math.max(0, p.stockQty)), costPrice: p.costPrice });
  lines.value.push({ key: ++seq, discountIsPct: true });
  toast.success(`أضيف ${low.length} صنف منخفض المخزون`);
}

const filled = computed(() => lines.value.filter((l) => l.productId));

function newLine(): Line {
  return { key: ++seq, discountIsPct: true };
}
function onLinesChange(next: Line[]) {
  lines.value = next;
}

function unitFactorOf(line: Line): number {
  const p = line.productId ? byId.value.get(line.productId) : undefined;
  return p?.units?.find((u) => u.id === line.unitId)?.factor ?? 1;
}

const lineTotal = (l: Line) => {
  const gross = num0(l.qty) * num0(l.costPrice);
  const disc = l.discountIsPct ? gross * (num0(l.discount) / 100) : num0(l.discount);
  return round2(Math.max(0, gross - disc));
};

const totals = computed(() =>
  computePurchaseTotals(
    filled.value.map((l) => ({ qty: num0(l.qty), costPrice: num0(l.costPrice), discount: l.discount, discountIsPct: l.discountIsPct, taxId: l.taxId })),
    invoiceDiscountAmount.value ? { amount: invoiceDiscountAmount.value } : undefined,
    settings.purchaseTaxRate,
  ),
);

const landedTotal = computed(() => round2(landedCosts.value.reduce((a, l) => a + num0(l.amount), 0)));
const grandTotalWithLanded = computed(() => round2(totals.value.grandTotal + landedTotal.value));

const noVatNumber = computed(() => selectedSupplier.value && !selectedSupplier.value.vatNumber);

const problems = computed(() => {
  const list: string[] = [];
  if (!supplierId.value) list.push('اختر المورد');
  if (!filled.value.length) list.push('أضف صنفاً واحداً على الأقل');
  if (filled.value.some((l) => !(num0(l.qty) > 0))) list.push('أدخل كمية صحيحة لكل صنف');
  if (filled.value.some((l) => num0(l.costPrice) < 0 || toNum(l.costPrice) === undefined)) list.push('أدخل سعر التكلفة لكل صنف');
  const ids = filled.value.map((l) => l.productId);
  if (new Set(ids).size !== ids.length) list.push('يوجد صنف مكرر');
  if (landedCosts.value.some((l) => !l.label.trim() || !(num0(l.amount) > 0))) list.push('أكمل بيانات التكاليف الإضافية (البند والمبلغ)');
  return list;
});

function buildInput(confirm: boolean) {
  return {
    supplierId: supplierId.value!,
    date: dateKeyToIso(date.value),
    note: note.value.trim() || undefined,
    confirm,
    supplierInvoiceNo: supplierInvoiceNo.value.trim() || undefined,
    supplierInvoiceDate: supplierInvoiceDate.value ? dateKeyToIso(supplierInvoiceDate.value) : undefined,
    invoiceDiscount: invoiceDiscountAmount.value ? { amount: invoiceDiscountAmount.value } : undefined,
    landedCosts: landedCosts.value
      .filter((l) => l.label.trim() && num0(l.amount) > 0)
      .map((l): LandedCostLineInput => ({ label: l.label.trim(), amount: num0(l.amount), supplierId: l.supplierId, spreadBy: l.spreadBy })),
    attachmentIds: [...attachmentIds.value],
    lines: filled.value.map((l) => ({
      productId: l.productId!,
      qty: num0(l.qty),
      costPrice: num0(l.costPrice),
      unitId: l.unitId,
      unitFactor: unitFactorOf(l),
      discount: l.discount,
      discountIsPct: l.discountIsPct,
      taxId: l.taxId || undefined,
    })),
  };
}

async function save(mode: 'draft' | 'send' | 'receive') {
  submitted.value = true;
  if (problems.value.length) return;
  saving.value = mode;
  try {
    const po = await savePurchaseOrder(buildInput(mode === 'receive'), id.value);
    if (mode === 'send') {
      await sendPurchaseOrderToSupplier(po.id);
      toast.success('تم إرسال أمر الشراء للمورد', po.number);
      router.push({ name: 'purchase', params: { id: po.id } });
      return;
    }
    posted.value = true;
    toast.success(mode === 'receive' ? 'تم تأكيد أمر الشراء واستلام البضاعة' : 'تم حفظ المسودة', po.number);
    router.push({ name: 'purchase', params: { id: po.id } });
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = null;
  }
}

const spreadOptions: { value: LandedCostSpread; label: string }[] = [
  { value: 'value', label: 'حسب القيمة' },
  { value: 'qty', label: 'حسب الكمية' },
];

// --- LineItemsEditor wiring (render-only — see PurchaseLinesGrid.vue) --------------------------
const lineColumns: LineColumn<Line>[] = [
  { key: 'productId', label: 'الصنف', type: 'custom' },
  { key: 'unit', label: 'الوحدة', type: 'custom', width: '110px' },
  { key: 'qty', label: 'الكمية', type: 'custom', width: '96px' },
  { key: 'costPrice', label: 'سعر التكلفة', type: 'custom', width: '112px' },
  { key: 'discount', label: 'الخصم', type: 'custom', width: '96px' },
  { key: 'taxId', label: 'الضريبة', type: 'custom', width: '144px' },
  { key: 'total', label: 'الإجمالي', type: 'custom' },
];

const totalsRows = computed<TotalsRow[]>(() => [
  { label: `صافي (${formatNumber(filled.value.length)} صنف)`, amount: totals.value.subTotal },
  { label: 'الضريبة', amount: totals.value.taxAmount },
  ...(landedTotal.value > 0 ? [{ label: 'تكاليف إضافية', amount: landedTotal.value }] : []),
  { label: 'الإجمالي المستحق', amount: grandTotalWithLanded.value, emphasis: true },
]);
</script>

<template>
  <ErrorState v-if="loadError" :message="loadError" />
  <FormPage v-else :title="id ? 'تعديل أمر شراء' : 'أمر شراء جديد'" :back="id ? { name: 'purchase', params: { id } } : { name: 'purchases' }">
    <template v-if="loading">
      <FormSection><SkeletonBlock :lines="8" height="h-8" /></FormSection>
    </template>
    <template v-else>
      <div v-if="noVatNumber" class="flex items-start gap-2 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2.5 text-xs text-warning">
        <AlertTriangle class="mt-0.5 size-4 shrink-0" />
        <span>هذا المورد بدون رقم ضريبي — ضريبة المدخلات لن تُحتسب مستردة وستُضاف إلى تكلفة البضاعة بدلاً من ذلك عند الاستلام.</span>
      </div>

      <FormSection :columns="2">
        <FormField label="المورد" required :error="submitted && !supplierId ? 'اختر المورد' : undefined">
          <AppCombobox v-model="supplierId" :options="supplierOptions" placeholder="اختر المورد…" />
        </FormField>
        <FormField label="التاريخ" required>
          <AppDatePicker v-model="date" />
        </FormField>
      </FormSection>

      <FormSection title="الأصناف">
        <div class="-mt-2 mb-2 flex justify-end">
          <AppButton size="sm" variant="ghost" :icon="PackageX" @click="addLowStock">إضافة الأصناف المنخفضة</AppButton>
        </div>
        <PurchaseLinesGrid
          :lines="lines"
          :columns="lineColumns"
          :new-line="newLine"
          :by-id="byId"
          :product-options="productOptions"
          :tax-options="taxOptions"
          :unit-options-for="unitOptionsFor"
          :line-total="lineTotal"
          @lines-change="onLinesChange"
          @product-selected="onProductSelected"
        />
      </FormSection>

      <FormSection title="تكاليف إضافية (شحن / جمارك / تخليص)">
        <div v-for="row in landedCosts" :key="row.key" class="grid grid-cols-[1fr_120px_1fr_140px_auto] items-end gap-2 border-b border-border pb-2 last:border-0">
          <AppInput v-model="row.label" label="البند" placeholder="شحن…" />
          <AppInput v-model.number="row.amount" type="number" min="0" step="0.01" label="المبلغ" />
          <AppCombobox v-model="row.supplierId" label="مورد آخر (اختياري)" :options="supplierOptions" clearable placeholder="نفس مورد الفاتورة" />
          <AppSelect v-model="row.spreadBy" label="طريقة التوزيع" :options="spreadOptions" />
          <button type="button" class="mb-1.5 rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" @click="landedCosts = landedCosts.filter((r) => r.key !== row.key)">
            <Trash class="size-3.5" />
          </button>
        </div>
        <AppButton size="sm" variant="ghost" :icon="Plus" @click="landedCosts.push({ key: ++lcSeq, label: '', spreadBy: 'value' })">إضافة بند تكلفة</AppButton>
        <p class="text-tiny text-text-secondary">تُضاف تكلفة كل بند إلى تكلفة أصناف المخزون حسب طريقة التوزيع المختارة؛ نفس المورد تُضاف لإجمالي المستحق له، ومورد آخر يُسجَّل كسطر مستحق منفصل.</p>
      </FormSection>

      <FormSection title="بيانات فاتورة المورد">
        <div v-if="!supplierInvoiceNo.trim() || !supplierInvoiceDate" class="flex items-start gap-2 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2 text-xs text-warning">
          <AlertTriangle class="mt-0.5 size-3.5 shrink-0" />
          <span>أدخل رقم فاتورة المورد وتاريخها بعد استلام البضاعة لإكمال المستند.</span>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <AppInput v-model="supplierInvoiceNo" label="رقم فاتورة المورد" ltr />
          <AppDatePicker v-model="supplierInvoiceDate" label="تاريخ فاتورة المورد" />
        </div>
        <AttachmentField :owner-ref="draftOwnerRef" />
      </FormSection>

      <template #aside>
        <TotalsPanel :rows="totalsRows" />
        <FormField label="خصم إضافي على الفاتورة">
          <AppInput v-model.number="invoiceDiscountAmount" type="number" min="0" step="0.01" placeholder="0" />
        </FormField>
        <FormField label="ملاحظات">
          <AppInput v-model="note" />
        </FormField>
        <ul v-if="submitted && problems.length" class="list-inside list-disc text-xs text-danger">
          <li v-for="p in problems" :key="p">{{ p }}</li>
        </ul>
        <p class="text-tiny leading-5 text-text-secondary">
          "إرسال للمورد" يحوّل الأمر لحالة "مرسل" ويطبع أمر الشراء؛ الاستلام يتم لاحقاً من شاشة الاستلام المخصّصة. "تأكيد واستلام الآن" يرحّل الكميات والقيد مباشرة.
        </p>
      </template>

      <template #actions>
        <FormActions>
          <template #primary>
            <AppButton variant="primary" :icon="PackageCheck" :loading="saving === 'receive'" :disabled="!!saving || posted" @click="save('receive')">تأكيد واستلام البضاعة الآن</AppButton>
          </template>
          <template #secondary>
            <AppButton :icon="Send" :loading="saving === 'send'" :disabled="!!saving || posted" @click="save('send')">إرسال للمورد (طباعة أمر شراء)</AppButton>
            <AppButton :icon="Save" :loading="saving === 'draft'" :disabled="!!saving || posted" @click="save('draft')">حفظ كمسودة</AppButton>
          </template>
        </FormActions>
      </template>
    </template>
  </FormPage>
</template>
