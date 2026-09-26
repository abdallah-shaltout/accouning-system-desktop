<script setup lang="ts">
// v2 §5 (docs/v2/14-platform.md §5 "Stock: ... stock adjustments, transfers, counts"): a photo of
// damaged goods or a supplier credit note can be attached while filling the adjustment.
// v2 doc 17 Phase F-2: migrated to FormPage + LineItemsEditor + TotalsPanel. Rendering-only change —
// every number here still comes from the same `change()`/`value()`/`gains`/`losses`/`journal`
// computeds as before; LineItemsEditor only renders rows and emits raw edits back.
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ClipboardCheck, PackageMinus, PackagePlus, ScanBarcode } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import FormActions from '@/modules/core/components/blocks/FormActions.vue';
import FormField from '@/modules/core/components/blocks/FormField.vue';
import FormPage from '@/modules/core/components/layouts/FormPage.vue';
import FormSection from '@/modules/core/components/blocks/FormSection.vue';
import JournalPreview from '@/modules/core/components/JournalPreview.vue';
import type { LineColumn } from '@/modules/core/components/blocks/LineItemsEditor.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import TotalsPanel, { type TotalsRow } from '@/modules/core/components/blocks/TotalsPanel.vue';
import { getAccounts, type AccountWithBalance } from '@/modules/accounting/services/accountingService';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, formatNumber, todayKey } from '@/modules/core/helpers/format';
import { num0, toNum } from '@/modules/core/helpers/numbers';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import ApprovalPinDialog from '../components/ApprovalPinDialog.vue';
import StockAdjustmentLinesGrid, { type AdjustmentLine } from '../components/StockAdjustmentLinesGrid.vue';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { createStockAdjustment } from '../services/inventoryService';
import { getProducts } from '../services/productService';
import type { Product, StockAdjustmentType, StockInReason } from '../types';

const STOCK_IN_REASON_OPTIONS: { value: StockInReason; label: string }[] = [
  { value: 'opening', label: 'رصيد افتتاحي' },
  { value: 'owner_contribution', label: 'مساهمة من المالك' },
  { value: 'gift', label: 'هدية / بضاعة مجانية من مورد' },
  { value: 'found', label: 'فائض تم العثور عليه' },
  { value: 'other', label: 'أخرى' },
];

type Line = AdjustmentLine;

const route = useRoute();
const router = useRouter();
const toast = useToast();
const catalog = useCatalogStore();
const settingsStore = useSettingsStore();

const initialType = String(route.query.type ?? 'STOCK_IN');
const type = ref<StockAdjustmentType>(['STOCK_IN', 'LOSS', 'STOCKTAKE'].includes(initialType) ? (initialType as StockAdjustmentType) : 'STOCK_IN');
const draftOwnerRef = `stock-adjustment:new:${Date.now()}`;
const date = ref(todayKey());
const note = ref('');
const lines = ref<Line[]>([]);
const products = ref<Product[]>([]);
const accounts = ref<AccountWithBalance[]>([]);
const loading = ref(true);
const saving = ref<'draft' | 'complete' | null>(null);
const stocktakeCategory = ref('');
const scan = ref('');
const lineErrors = ref<Record<number, string>>({});
// A3: every STOCK_IN needs a reason — it decides the credit account (see backend/inventory.ts).
const stockInReason = ref<StockInReason>('opening');
const offsetAccountId = ref('');
// v2 §5: stock-in/write-off above settings.inventoryApprovalThreshold needs a manager PIN.
const approvalOpen = ref(false);
const approvedBy = ref('');
let seq = 0;

const offsetAccountOptions = computed(() =>
  accounts.value
    .filter((a) => a.active && !a.isGroup && a.allowManual && a.systemRole !== 'receivable' && a.systemRole !== 'payable')
    .map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` })),
);

const byId = computed(() => new Map(products.value.map((p) => [p.id, p])));
const productOptions = computed(() =>
  products.value.map((p) => ({
    value: p.id,
    label: p.name,
    sublabel: `${p.sku} · المتوفر ${formatNumber(p.stockQty)}`,
    keywords: `${p.sku} ${p.barcode ?? ''}`,
  })),
);

onMounted(async () => {
  await catalog.load();
  const [prods, accs] = await Promise.all([getProducts({ type: 'product' }), getAccounts(), settingsStore.load()]);
  products.value = prods.filter((p) => p.stockMode !== 'none');
  accounts.value = accs;
  loading.value = false;
  const pre = typeof route.query.product === 'string' ? route.query.product : undefined;
  if (type.value === 'STOCKTAKE') loadStocktake();
  else lines.value = [{ key: ++seq, productId: pre && byId.value.has(pre) ? pre : undefined }];
});

watch(type, (t) => {
  router.replace({ query: { type: t } });
  lineErrors.value = {};
  // Stocktake pre-fills every product; the other types start from one empty line.
  if (t === 'STOCKTAKE') loadStocktake();
  else lines.value = [{ key: ++seq }];
});

function loadStocktake() {
  lines.value = products.value
    .filter((p) => !stocktakeCategory.value || p.categoryId === stocktakeCategory.value)
    .map((p) => ({ key: ++seq, productId: p.id, counted: p.stockQty }));
}
watch(stocktakeCategory, () => type.value === 'STOCKTAKE' && loadStocktake());

function newLine(): Line {
  return { key: ++seq };
}

function addLine(productId?: string) {
  const existing = productId && lines.value.find((l) => l.productId === productId);
  if (existing) {
    existing.qty = num0(existing.qty) + 1;
    return;
  }
  const empty = lines.value.find((l) => !l.productId);
  if (empty && productId) {
    empty.productId = productId;
    empty.qty = 1;
  } else lines.value.push({ key: ++seq, productId, qty: productId ? 1 : undefined });
}

function onScan() {
  const code = scan.value.trim();
  if (!code) return;
  const product = products.value.find((p) => p.barcode === code || p.sku.toLowerCase() === code.toLowerCase());
  if (!product) toast.warning('لم يتم العثور على منتج', code);
  else if (type.value === 'STOCKTAKE') {
    const line = lines.value.find((l) => l.productId === product.id);
    if (line) line.counted = num0(line.counted) + 1;
    else lines.value.push({ key: ++seq, productId: product.id, counted: 1 });
  } else addLine(product.id);
  scan.value = '';
}

function change(line: Line): number {
  const p = line.productId ? byId.value.get(line.productId) : undefined;
  if (!p) return 0;
  if (type.value === 'STOCKTAKE') {
    const counted = toNum(line.counted);
    return counted === undefined ? 0 : counted - p.stockQty;
  }
  return type.value === 'LOSS' ? -num0(line.qty) : num0(line.qty);
}

function value(line: Line): number {
  const p = line.productId ? byId.value.get(line.productId) : undefined;
  return p ? change(line) * p.costPrice : 0;
}

const gains = computed(() => lines.value.reduce((a, l) => a + Math.max(0, value(l)), 0));
const losses = computed(() => lines.value.reduce((a, l) => a + Math.max(0, -value(l)), 0));
const diffCount = computed(() => lines.value.filter((l) => change(l) !== 0).length);

const inventoryAccount = computed(() => accounts.value.find((a) => a.systemRole === 'inventory'));
const offsetAccount = computed(() => {
  if (stockInReason.value === 'other') return accounts.value.find((a) => a.id === offsetAccountId.value);
  const role = stockInReason.value === 'opening' ? 'openingBalanceEquity' : stockInReason.value === 'owner_contribution' ? 'ownerCurrent' : stockInReason.value === 'gift' ? 'otherIncome' : 'inventoryVariance';
  return accounts.value.find((a) => a.systemRole === role);
});

const journal = computed(() => {
  const inv = inventoryAccount.value;
  const out: { accountCode: string; accountName: string; debit: number; credit: number }[] = [];
  const push = (a: AccountWithBalance | undefined, debit: number, credit: number) => a && (debit || credit) && out.push({ accountCode: a.code, accountName: a.name, debit, credit });
  if (type.value === 'STOCK_IN') {
    push(inv, gains.value, 0);
    push(offsetAccount.value, 0, gains.value);
  } else if (type.value === 'LOSS') {
    // Write-off (damaged/expired): its own account (5120), not the stocktake-variance account.
    const writeOff = accounts.value.find((a) => a.systemRole === 'inventoryWriteOff');
    push(writeOff, losses.value, 0);
    push(inv, 0, losses.value);
  } else {
    // A4: stocktake gains AND losses both post to inventoryVariance — corrections of COGS.
    const variance = accounts.value.find((a) => a.systemRole === 'inventoryVariance');
    push(inv, gains.value, 0);
    push(variance, 0, gains.value);
    push(variance, losses.value, 0);
    push(inv, 0, losses.value);
  }
  return out;
});

const typeOptions = [
  { value: 'STOCK_IN' as const, label: 'إدخال مخزون', icon: PackagePlus },
  { value: 'LOSS' as const, label: 'إتلاف / فقد', icon: PackageMinus },
  { value: 'STOCKTAKE' as const, label: 'جرد', icon: ClipboardCheck },
];

const typeHelp = computed(
  () =>
    ({
      STOCK_IN: 'إضافة بضاعة للمخزون بدون أمر شراء — اختر السبب أدناه ليحدد الحساب المقابل.',
      LOSS: 'إخراج بضاعة تالفة أو مفقودة من المخزون. تُسجل في حساب البضاعة التالفة ومنتهية الصلاحية.',
      STOCKTAKE: 'أدخل الكمية المعدودة فعلياً لكل صنف — يُحتسب الفرق تلقائياً ويُسجل في حساب فروقات جرد المخزون (ربحاً كان أو خسارة).',
    })[type.value],
);

function validateLines(): boolean {
  const errs: Record<number, string> = {};
  const filled = lines.value.filter((l) => l.productId);
  for (const l of filled) {
    const p = byId.value.get(l.productId!)!;
    if (type.value === 'STOCKTAKE') {
      const counted = toNum(l.counted);
      if (counted === undefined || counted < 0) errs[l.key] = 'أدخل الكمية المعدودة';
    } else if (!(num0(l.qty) > 0)) errs[l.key] = 'أدخل كمية صحيحة';
    else if (type.value === 'LOSS' && num0(l.qty) > p.stockQty) errs[l.key] = `المتوفر ${p.stockQty} فقط`;
    else if (type.value === 'STOCK_IN' && p.trackBatches && !l.batchNo?.trim()) errs[l.key] = 'أدخل رقم التشغيلة';
  }
  const ids = filled.map((l) => l.productId);
  filled.forEach((l) => {
    if (ids.indexOf(l.productId) !== ids.lastIndexOf(l.productId)) errs[l.key] = 'الصنف مكرر';
  });
  lineErrors.value = errs;
  if (!filled.length) toast.warning('أضف صنفاً واحداً على الأقل');
  if (type.value === 'STOCK_IN' && stockInReason.value === 'other' && !offsetAccountId.value) {
    toast.warning('اختر الحساب المقابل لسبب "أخرى"');
    return false;
  }
  return filled.length > 0 && !Object.keys(errs).length;
}

const pendingValue = computed(() => (type.value === 'STOCK_IN' ? gains.value : type.value === 'LOSS' ? losses.value : 0));
const approvalThreshold = computed(() => settingsStore.settings?.inventoryApprovalThreshold ?? 0);
const needsApproval = computed(
  () => type.value !== 'STOCKTAKE' && approvalThreshold.value > 0 && pendingValue.value >= approvalThreshold.value && !approvedBy.value,
);

async function submit(asDraft: boolean) {
  if (!validateLines()) return;
  if (!asDraft && needsApproval.value) {
    approvalOpen.value = true;
    return;
  }
  saving.value = asDraft ? 'draft' : 'complete';
  try {
    // A stocktake keeps every counted line (zero differences included) as the record of the count.
    const filled = lines.value.filter((l) => l.productId);
    const adj = await createStockAdjustment(
      {
        type: type.value,
        date: dateKeyToIso(date.value),
        note: note.value.trim() || undefined,
        reason: type.value === 'STOCK_IN' ? stockInReason.value : undefined,
        offsetAccountId: type.value === 'STOCK_IN' && stockInReason.value === 'other' ? offsetAccountId.value : undefined,
        lines: filled.map((l) =>
          type.value === 'STOCKTAKE'
            ? { productId: l.productId!, countedQty: toNum(l.counted) }
            : { productId: l.productId!, qtyChange: toNum(l.qty), batchNo: l.batchNo, expiryDate: l.expiryDate },
        ),
        approvedBy: asDraft ? undefined : approvedBy.value || undefined,
      },
      asDraft,
    );
    toast.success(asDraft ? 'تم حفظ المسودة' : 'تم اعتماد التسوية', adj.number);
    router.push({ name: 'adjustment', params: { id: adj.id } });
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = null;
  }
}

function onApproved(userId: string) {
  approvedBy.value = userId;
  submit(false);
}

// --- LineItemsEditor wiring (render-only — see file header) -----------------------------------
const columns = computed<LineColumn<Line>[]>(() => {
  const cols: LineColumn<Line>[] = [{ key: 'productId', label: 'الصنف', type: 'custom' }];
  cols.push({ key: 'systemQty', label: type.value === 'STOCKTAKE' ? 'رصيد النظام' : 'المتوفر', type: 'custom', width: '110px' });
  cols.push({ key: type.value === 'STOCKTAKE' ? 'counted' : 'qty', label: type.value === 'STOCKTAKE' ? 'المعدود' : 'الكمية', type: 'custom', width: '110px' });
  if (type.value === 'STOCKTAKE') cols.push({ key: 'diff', label: 'الفرق', type: 'custom', width: '100px' });
  if (type.value === 'STOCK_IN') cols.push({ key: 'batchNo', label: 'التشغيلة / الصلاحية', type: 'custom', width: '150px' });
  cols.push({ key: 'costPrice', label: 'التكلفة', type: 'custom', width: '110px' });
  cols.push({ key: 'value', label: 'القيمة', type: 'custom', width: '110px' });
  return cols;
});

function onLinesChange(next: Line[]) {
  lines.value = next;
}
</script>

<template>
  <FormPage :title="'تسوية مخزون جديدة'" :back="{ name: 'adjustments' }">
    <template v-if="loading">
      <FormSection><SkeletonBlock :lines="8" height="h-8" /></FormSection>
    </template>
    <template v-else>
      <FormSection>
        <div class="flex flex-wrap items-end gap-4">
          <div>
            <span class="field-label">نوع التسوية</span>
            <SegmentedControl v-model="type" :options="typeOptions" />
          </div>
          <AppDatePicker v-model="date" label="التاريخ" class="w-40" />
          <AppSelect
            v-if="type === 'STOCKTAKE'"
            v-model="stocktakeCategory"
            label="نطاق الجرد"
            class="w-44"
            placeholder="كل الأصناف"
            :options="catalog.categories.filter((c) => c.id !== 'cat-services').map((c) => ({ value: c.id, label: c.name }))"
          />
          <AppSelect v-if="type === 'STOCK_IN'" v-model="stockInReason" label="سبب الإدخال" class="w-56" :options="STOCK_IN_REASON_OPTIONS" />
          <AppSelect
            v-if="type === 'STOCK_IN' && stockInReason === 'other'"
            v-model="offsetAccountId"
            label="الحساب المقابل"
            class="w-56"
            placeholder="اختر الحساب…"
            :options="offsetAccountOptions"
          />
        </div>
        <p class="text-xs leading-5 text-text-secondary">{{ typeHelp }}</p>
      </FormSection>

      <FormSection title="الأصناف">
        <template #default>
          <div class="-mt-2 mb-2 flex justify-end">
            <div class="w-64">
              <AppInput v-model="scan" placeholder="امسح الباركود أو اكتب SKU ثم Enter" @keydown.enter.prevent="onScan">
                <template #prefix><ScanBarcode class="size-4" /></template>
              </AppInput>
            </div>
          </div>
          <div class="max-h-[58vh] overflow-y-auto">
            <StockAdjustmentLinesGrid
              :lines="lines"
              :columns="columns"
              :new-line="newLine"
              :type="type"
              :by-id="byId"
              :product-options="productOptions"
              :line-errors="lineErrors"
              :change="change"
              :value="value"
              @lines-change="onLinesChange"
            />
          </div>
        </template>
      </FormSection>

      <template #aside>
        <TotalsPanel
          :rows="([
            type !== 'STOCKTAKE' ? { label: type === 'STOCK_IN' ? 'قيمة البضاعة المدخلة' : 'زيادات الجرد', amount: gains } : null,
            type !== 'STOCK_IN' ? { label: type === 'LOSS' ? 'قيمة التالف' : 'عجز الجرد', amount: losses } : null,
          ].filter(Boolean) as TotalsRow[])"
        />
        <p v-if="type === 'STOCKTAKE'" class="text-body text-text-secondary">
          أصناف بها فروقات: <span class="num">{{ formatNumber(diffCount) }}</span> من <span class="num">{{ formatNumber(lines.length) }}</span>
        </p>

        <FormField label="البيان / ملاحظات">
          <AppInput v-model="note" placeholder="مثال: جرد نهاية الشهر" />
        </FormField>

        <FormField label="المرفقات">
          <AttachmentField :owner-ref="draftOwnerRef" />
        </FormField>

        <JournalPreview v-if="journal.length" :lines="journal" title="القيد المحاسبي المتوقع" />
      </template>

      <template #actions>
        <FormActions>
          <template #primary>
            <AppButton variant="primary" :loading="saving === 'complete'" :disabled="!!saving" @click="submit(false)">اعتماد التسوية</AppButton>
          </template>
          <template #secondary>
            <AppButton :loading="saving === 'draft'" :disabled="!!saving" @click="submit(true)">حفظ كمسودة</AppButton>
          </template>
        </FormActions>
      </template>
    </template>

    <ApprovalPinDialog v-model:open="approvalOpen" :value="pendingValue" :threshold="approvalThreshold" @approved="onApproved" />
  </FormPage>
</template>
