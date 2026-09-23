<script setup lang="ts">
// TODO(phase 6): wire <AttachmentField> onto stock adjustments/transfers/counts (docs/v2/14-platform.md §5).
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ClipboardCheck, PackageMinus, PackagePlus, Plus, ScanBarcode, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import JournalPreview from '@/modules/core/components/JournalPreview.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, formatNumber, todayKey } from '@/modules/core/helpers/format';
import { num0, toNum } from '@/modules/core/helpers/numbers';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { createStockAdjustment } from '../services/inventoryService';
import { getProducts } from '../services/productService';
import type { Product, StockAdjustmentType } from '../types';

interface Line {
  key: number;
  productId?: string;
  qty?: number;
  counted?: number;
}

const route = useRoute();
const router = useRouter();
const toast = useToast();
const catalog = useCatalogStore();

const initialType = String(route.query.type ?? 'STOCK_IN');
const type = ref<StockAdjustmentType>(['STOCK_IN', 'LOSS', 'STOCKTAKE'].includes(initialType) ? (initialType as StockAdjustmentType) : 'STOCK_IN');
const date = ref(todayKey());
const note = ref('');
const lines = ref<Line[]>([]);
const products = ref<Product[]>([]);
const loading = ref(true);
const saving = ref<'draft' | 'complete' | null>(null);
const stocktakeCategory = ref('');
const scan = ref('');
const lineErrors = ref<Record<number, string>>({});
let seq = 0;

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
  products.value = await getProducts({ type: 'product' });
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

const journal = computed(() => {
  const acc = (code: string) => ({ '1140': 'بضاعة المخزون', '3100': 'رأس المال', '4400': 'أرباح جرد المخزون', '5800': 'خسائر جرد المخزون' })[code]!;
  const out: { accountCode: string; accountName: string; debit: number; credit: number }[] = [];
  const push = (code: string, debit: number, credit: number) => (debit || credit) && out.push({ accountCode: code, accountName: acc(code), debit, credit });
  if (type.value === 'STOCK_IN') {
    push('1140', gains.value, 0);
    push('3100', 0, gains.value);
  } else {
    push('1140', gains.value, 0);
    push('4400', 0, gains.value);
    push('5800', losses.value, 0);
    push('1140', 0, losses.value);
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
      STOCK_IN: 'إضافة بضاعة للمخزون بدون أمر شراء (رصيد افتتاحي أو بضاعة من المالك). تُسجل مقابل رأس المال.',
      LOSS: 'إخراج بضاعة تالفة أو مفقودة من المخزون. تُسجل كخسارة بسعر التكلفة.',
      STOCKTAKE: 'أدخل الكمية المعدودة فعلياً لكل صنف — يُحتسب الفرق تلقائياً ويُسجل كربح أو خسارة جرد.',
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
  }
  const ids = filled.map((l) => l.productId);
  filled.forEach((l) => {
    if (ids.indexOf(l.productId) !== ids.lastIndexOf(l.productId)) errs[l.key] = 'الصنف مكرر';
  });
  lineErrors.value = errs;
  if (!filled.length) toast.warning('أضف صنفاً واحداً على الأقل');
  return filled.length > 0 && !Object.keys(errs).length;
}

async function submit(asDraft: boolean) {
  if (!validateLines()) return;
  saving.value = asDraft ? 'draft' : 'complete';
  try {
    // A stocktake keeps every counted line (zero differences included) as the record of the count.
    const filled = lines.value.filter((l) => l.productId);
    const adj = await createStockAdjustment(
      {
        type: type.value,
        date: dateKeyToIso(date.value),
        note: note.value.trim() || undefined,
        lines: filled.map((l) => (type.value === 'STOCKTAKE' ? { productId: l.productId!, countedQty: toNum(l.counted) } : { productId: l.productId!, qtyChange: toNum(l.qty) })),
      },
      asDraft,
    );
    toast.success(asDraft ? 'تم حفظ المسودة' : 'تم اعتماد التسوية', adj.number);
    router.push(`/inventory/adjustments/${adj.id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = null;
  }
}
</script>

<template>
  <div>
    <PageHeader title="تسوية مخزون جديدة" back="/inventory/adjustments" />

    <div class="grid items-start gap-5 xl:grid-cols-[1fr_340px]">
      <div class="space-y-5">
        <AppCard padding="sm">
          <div class="flex flex-wrap items-end gap-4">
            <div>
              <span class="field-label">نوع التسوية</span>
              <SegmentedControl v-model="type" :options="typeOptions" />
            </div>
            <AppInput v-model="date" type="date" label="التاريخ" class="w-40" />
            <AppSelect
              v-if="type === 'STOCKTAKE'"
              v-model="stocktakeCategory"
              label="نطاق الجرد"
              class="w-44"
              placeholder="كل الأصناف"
              :options="catalog.categories.filter((c) => c.id !== 'cat-services').map((c) => ({ value: c.id, label: c.name }))"
            />
          </div>
          <p class="mt-3 text-xs leading-5 text-text-secondary">{{ typeHelp }}</p>
        </AppCard>

        <AppCard padding="none">
          <div class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3">
            <h2 class="text-body font-semibold">الأصناف</h2>
            <div class="relative w-64">
              <ScanBarcode class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-text-secondary" />
              <input v-model="scan" class="control ps-8" placeholder="امسح الباركود أو اكتب SKU ثم Enter" @keydown.enter.prevent="onScan" />
            </div>
          </div>

          <div v-if="loading" class="p-4"><SkeletonBlock :lines="5" height="h-8" /></div>
          <EmptyState v-else-if="!lines.length" title="لا توجد أصناف في هذا النطاق" compact />
          <div v-else class="max-h-[58vh] overflow-y-auto">
            <table class="w-full text-body">
              <thead class="sticky top-0 z-[1] bg-surface text-xs text-text-secondary">
                <tr class="border-b border-border">
                  <th class="px-4 py-2 text-start font-medium">الصنف</th>
                  <th class="px-2 py-2 text-start font-medium">{{ type === 'STOCKTAKE' ? 'رصيد النظام' : 'المتوفر' }}</th>
                  <th class="px-2 py-2 text-start font-medium">{{ type === 'STOCKTAKE' ? 'المعدود' : 'الكمية' }}</th>
                  <th v-if="type === 'STOCKTAKE'" class="px-2 py-2 text-start font-medium">الفرق</th>
                  <th class="px-2 py-2 text-start font-medium">التكلفة</th>
                  <th class="px-2 py-2 text-start font-medium">القيمة</th>
                  <th v-if="type !== 'STOCKTAKE'" class="w-10" />
                </tr>
              </thead>
              <tbody>
                <tr v-for="line in lines" :key="line.key" class="border-b border-border last:border-0" :class="type === 'STOCKTAKE' && change(line) !== 0 && 'bg-warning/5'">
                  <td class="min-w-56 px-4 py-1.5">
                    <AppCombobox
                      v-if="type !== 'STOCKTAKE'"
                      v-model="line.productId"
                      :options="productOptions"
                      placeholder="اختر صنفاً…"
                      search-placeholder="اسم، SKU، أو باركود"
                      dense
                    />
                    <span v-else>
                      {{ byId.get(line.productId!)?.name }}
                      <span class="num block text-tiny text-text-secondary">{{ byId.get(line.productId!)?.sku }}</span>
                    </span>
                  </td>
                  <td class="px-2 py-1.5"><span class="num text-text-secondary">{{ line.productId ? formatNumber(byId.get(line.productId)?.stockQty) : '—' }}</span></td>
                  <td class="px-2 py-1.5">
                    <input
                      v-if="type === 'STOCKTAKE'"
                      v-model.number="line.counted"
                      type="number"
                      min="0"
                      class="control h-8 w-24"
                      :aria-invalid="!!lineErrors[line.key] || undefined"
                      @focus="($event.target as HTMLInputElement).select()"
                    />
                    <input
                      v-else
                      v-model.number="line.qty"
                      type="number"
                      min="0"
                      class="control h-8 w-24"
                      :aria-invalid="!!lineErrors[line.key] || undefined"
                    />
                    <p v-if="lineErrors[line.key]" class="mt-0.5 text-tiny text-danger">{{ lineErrors[line.key] }}</p>
                  </td>
                  <td v-if="type === 'STOCKTAKE'" class="px-2 py-1.5">
                    <span class="num font-medium" :class="change(line) > 0 ? 'text-success' : change(line) < 0 ? 'text-danger' : 'text-text-secondary'">
                      {{ change(line) > 0 ? '+' : '' }}{{ formatNumber(change(line)) }}
                    </span>
                  </td>
                  <td class="px-2 py-1.5"><MoneyText v-if="line.productId" :value="byId.get(line.productId)?.costPrice" plain class="text-text-secondary" /></td>
                  <td class="px-2 py-1.5"><MoneyText :value="value(line)" plain :signed="type === 'STOCKTAKE'" dash-zero /></td>
                  <td v-if="type !== 'STOCKTAKE'" class="px-2">
                    <button
                      type="button"
                      class="rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger"
                      aria-label="حذف السطر"
                      @click="lines = lines.filter((l) => l.key !== line.key)"
                    >
                      <Trash class="size-3.5" />
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-if="type !== 'STOCKTAKE'" class="border-t border-border px-4 py-2">
            <AppButton size="sm" variant="ghost" :icon="Plus" @click="addLine()">إضافة صنف</AppButton>
          </div>
        </AppCard>
      </div>

      <div class="space-y-4 xl:sticky xl:top-0">
        <AppCard title="الملخص" padding="sm">
          <dl class="space-y-2 text-body">
            <div v-if="type === 'STOCKTAKE'" class="flex justify-between">
              <dt class="text-text-secondary">أصناف بها فروقات</dt>
              <dd><span class="num">{{ formatNumber(diffCount) }}</span> من <span class="num">{{ formatNumber(lines.length) }}</span></dd>
            </div>
            <div v-if="type !== 'LOSS'" class="flex justify-between">
              <dt class="text-text-secondary">{{ type === 'STOCK_IN' ? 'قيمة البضاعة المدخلة' : 'زيادات الجرد' }}</dt>
              <dd><MoneyText :value="gains" /></dd>
            </div>
            <div v-if="type !== 'STOCK_IN'" class="flex justify-between">
              <dt class="text-text-secondary">{{ type === 'LOSS' ? 'قيمة التالف' : 'عجز الجرد' }}</dt>
              <dd><MoneyText :value="losses" /></dd>
            </div>
          </dl>
          <div class="mt-4">
            <AppInput v-model="note" label="البيان / ملاحظات" placeholder="مثال: جرد نهاية الشهر" />
          </div>
        </AppCard>

        <JournalPreview v-if="journal.length" :lines="journal" title="القيد المحاسبي المتوقع" />

        <div class="flex gap-2">
          <AppButton variant="primary" class="flex-1" :loading="saving === 'complete'" :disabled="!!saving" @click="submit(false)">اعتماد التسوية</AppButton>
          <AppButton :loading="saving === 'draft'" :disabled="!!saving" @click="submit(true)">حفظ كمسودة</AppButton>
        </div>
      </div>
    </div>
  </div>
</template>
