<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { AlertTriangle, PackageCheck, PackageX, Plus, Save, Send, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useGridTab } from '@/modules/core/controllers/useGridTab';
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
import { getPurchaseOrder, savePurchaseOrder, sendPurchaseOrderToSupplier } from '../services/purchaseService';
import type { LandedCostLineInput, LandedCostSpread } from '../types';

interface Line {
  key: number;
  productId?: string;
  unitId?: string;
  qty?: number;
  costPrice?: number;
  discount?: number;
  discountIsPct?: boolean;
  taxId?: string;
}

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

const linesBody = ref<HTMLElement>();
const onLinesKeydown = useGridTab({
  container: linesBody,
  addRow: () => lines.value.push({ key: ++seq, discountIsPct: true }),
  isFilled: (i) => !!lines.value[i]?.productId,
});

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
</script>

<template>
  <div>
    <PageHeader :title="id ? 'تعديل أمر شراء' : 'أمر شراء جديد'" :back="id ? { name: 'purchase', params: { id } } : { name: 'purchases' }" />

    <ErrorState v-if="loadError" :message="loadError" />
    <AppCard v-else-if="loading"><SkeletonBlock :lines="8" height="h-8" /></AppCard>
    <div v-else class="grid items-start gap-5 xl:grid-cols-[1fr_340px]">
      <div class="space-y-5">
        <div v-if="noVatNumber" class="flex items-start gap-2 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2.5 text-xs text-warning">
          <AlertTriangle class="mt-0.5 size-4 shrink-0" />
          <span>هذا المورد بدون رقم ضريبي — ضريبة المدخلات لن تُحتسب مستردة وستُضاف إلى تكلفة البضاعة بدلاً من ذلك عند الاستلام.</span>
        </div>

        <AppCard padding="sm">
          <div class="grid gap-4 sm:grid-cols-[1fr_160px]">
            <AppCombobox
              v-model="supplierId"
              label="المورد"
              required
              :options="supplierOptions"
              placeholder="اختر المورد…"
              :error="submitted && !supplierId ? 'اختر المورد' : undefined"
            />
            <AppDatePicker v-model="date" label="التاريخ" required />
          </div>
        </AppCard>

        <AppCard padding="none">
          <template #actions>
            <AppButton size="sm" variant="ghost" :icon="PackageX" @click="addLowStock">إضافة الأصناف المنخفضة</AppButton>
          </template>
          <table class="w-full text-body">
            <thead class="bg-surface text-xs text-text-secondary">
              <tr class="border-b border-border">
                <th class="px-4 py-2 text-start font-medium">الصنف</th>
                <th class="px-2 py-2 text-start font-medium">الوحدة</th>
                <th class="w-24 px-2 py-2 text-start font-medium">الكمية</th>
                <th class="w-28 px-2 py-2 text-start font-medium">سعر التكلفة</th>
                <th class="w-24 px-2 py-2 text-start font-medium">الخصم</th>
                <th class="w-36 px-2 py-2 text-start font-medium">الضريبة</th>
                <th class="px-2 py-2 text-start font-medium">الإجمالي</th>
                <th class="w-10" />
              </tr>
            </thead>
            <tbody ref="linesBody" @keydown="onLinesKeydown">
              <tr v-for="line in lines" :key="line.key" class="border-b border-border last:border-0">
                <td class="min-w-52 px-4 py-1.5">
                  <AppCombobox
                    v-model="line.productId"
                    :options="productOptions"
                    placeholder="اختر صنفاً…"
                    search-placeholder="اسم أو SKU أو باركود"
                    dense
                    @select="onProductSelected(line)"
                  />
                </td>
                <td class="px-2 py-1.5">
                  <select v-if="unitOptionsFor(line.productId).length" v-model="line.unitId" class="control h-8 text-xs">
                    <option v-for="o in unitOptionsFor(line.productId)" :key="o.value" :value="o.value">{{ o.label }}</option>
                  </select>
                  <span v-else class="text-tiny text-text-secondary">أساسية</span>
                </td>
                <td class="px-2 py-1.5"><input v-model.number="line.qty" type="number" min="0.01" step="any" class="control num h-8" /></td>
                <td class="px-2 py-1.5"><input v-model.number="line.costPrice" type="number" min="0" step="0.01" class="control num h-8" /></td>
                <td class="px-2 py-1.5">
                  <div class="flex gap-1">
                    <input v-model.number="line.discount" type="number" min="0" step="0.01" class="control num h-8 w-14" placeholder="0" />
                    <button type="button" class="rounded border border-border px-1.5 text-caption text-text-secondary hover:bg-surface-hover" @click="line.discountIsPct = !line.discountIsPct">
                      {{ line.discountIsPct ? '%' : 'ر.س' }}
                    </button>
                  </div>
                </td>
                <td class="px-2 py-1.5">
                  <select v-model="line.taxId" class="control h-8 text-xs">
                    <option v-for="o in taxOptions" :key="o.value" :value="o.value">{{ o.label }}</option>
                  </select>
                </td>
                <td class="px-2 py-1.5"><MoneyText :value="lineTotal(line)" plain dash-zero /></td>
                <td class="px-2">
                  <button
                    type="button"
                    class="rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger disabled:opacity-30"
                    :disabled="lines.length <= 1"
                    aria-label="حذف السطر"
                    data-grid-skip
                    @click="lines = lines.filter((l) => l.key !== line.key)"
                  >
                    <Trash class="size-3.5" />
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <div class="border-t border-border px-4 py-2">
            <AppButton size="sm" variant="ghost" :icon="Plus" @click="lines.push({ key: ++seq, discountIsPct: true })">إضافة صنف</AppButton>
          </div>
        </AppCard>

        <AppCard title="تكاليف إضافية (شحن / جمارك / تخليص)" padding="sm">
          <div v-for="row in landedCosts" :key="row.key" class="mb-2 grid grid-cols-[1fr_120px_1fr_140px_auto] items-end gap-2 border-b border-border pb-2 last:border-0">
            <AppInput v-model="row.label" label="البند" placeholder="شحن…" />
            <AppInput v-model.number="row.amount" type="number" min="0" step="0.01" label="المبلغ" />
            <AppCombobox v-model="row.supplierId" label="مورد آخر (اختياري)" :options="supplierOptions" clearable placeholder="نفس مورد الفاتورة" />
            <AppSelect v-model="row.spreadBy" label="طريقة التوزيع" :options="spreadOptions" />
            <button type="button" class="mb-1.5 rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" @click="landedCosts = landedCosts.filter((r) => r.key !== row.key)">
              <Trash class="size-3.5" />
            </button>
          </div>
          <AppButton size="sm" variant="ghost" :icon="Plus" @click="landedCosts.push({ key: ++lcSeq, label: '', spreadBy: 'value' })">إضافة بند تكلفة</AppButton>
          <p class="mt-2 text-tiny text-text-secondary">تُضاف تكلفة كل بند إلى تكلفة أصناف المخزون حسب طريقة التوزيع المختارة؛ نفس المورد تُضاف لإجمالي المستحق له، ومورد آخر يُسجَّل كسطر مستحق منفصل.</p>
        </AppCard>

        <AppCard title="بيانات فاتورة المورد" padding="sm">
          <div v-if="!supplierInvoiceNo.trim() || !supplierInvoiceDate" class="mb-3 flex items-start gap-2 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2 text-xs text-warning">
            <AlertTriangle class="mt-0.5 size-3.5 shrink-0" />
            <span>أدخل رقم فاتورة المورد وتاريخها بعد استلام البضاعة لإكمال المستند.</span>
          </div>
          <div class="grid gap-3 sm:grid-cols-2">
            <AppInput v-model="supplierInvoiceNo" label="رقم فاتورة المورد" ltr />
            <AppDatePicker v-model="supplierInvoiceDate" label="تاريخ فاتورة المورد" />
          </div>
          <div class="mt-3">
            <AttachmentField :owner-ref="draftOwnerRef" />
          </div>
        </AppCard>
      </div>

      <div class="space-y-4 xl:sticky xl:top-0">
        <AppCard title="الإجمالي" padding="sm">
          <dl class="space-y-1.5 text-body">
            <div class="flex justify-between"><dt class="text-text-secondary">صافي ({{ formatNumber(filled.length) }} صنف)</dt><dd><MoneyText :value="totals.subTotal" /></dd></div>
            <div><AppInput v-model.number="invoiceDiscountAmount" type="number" min="0" step="0.01" label="خصم إضافي على الفاتورة" placeholder="0" /></div>
            <div class="flex justify-between"><dt class="text-text-secondary">الضريبة</dt><dd><MoneyText :value="totals.taxAmount" /></dd></div>
            <div v-if="landedTotal > 0" class="flex justify-between"><dt class="text-text-secondary">تكاليف إضافية</dt><dd><MoneyText :value="landedTotal" /></dd></div>
            <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>الإجمالي المستحق</dt><dd><MoneyText :value="grandTotalWithLanded" /></dd></div>
          </dl>
          <div class="mt-4"><AppInput v-model="note" label="ملاحظات" /></div>
        </AppCard>
        <ul v-if="submitted && problems.length" class="list-inside list-disc text-xs text-danger">
          <li v-for="p in problems" :key="p">{{ p }}</li>
        </ul>
        <AppButton variant="primary" block :icon="PackageCheck" :loading="saving === 'receive'" :disabled="!!saving || posted" @click="save('receive')">تأكيد واستلام البضاعة الآن</AppButton>
        <AppButton block :icon="Send" :loading="saving === 'send'" :disabled="!!saving || posted" @click="save('send')">إرسال للمورد (طباعة أمر شراء)</AppButton>
        <AppButton block :icon="Save" :loading="saving === 'draft'" :disabled="!!saving || posted" @click="save('draft')">حفظ كمسودة</AppButton>
        <p class="text-tiny leading-5 text-text-secondary">
          "إرسال للمورد" يحوّل الأمر لحالة "مرسل" ويطبع أمر الشراء؛ الاستلام يتم لاحقاً من شاشة الاستلام المخصّصة. "تأكيد واستلام الآن" يرحّل الكميات والقيد مباشرة.
        </p>
      </div>
    </div>
  </div>
</template>
