<script setup lang="ts">
/**
 * Label builder (docs/v2/07-products-and-inventory.md §6 "Barcode / QR label printing
 * (/catalog/labels)", docs/v2/12-documents-pdf-excel.md §3-4). Entry points per the doc: the
 * product list (bulk select — not built yet, no bulk-select UI exists on ProductListPage), the
 * product page ("طباعة ملصقات"), and a purchase receipt's "طباعة ملصقات للكميات المستلمة" (Phase
 * 8's `PurchaseReceivePage.vue` TODO(phase 11b) stub, wired below). All three land here with
 * optional query params that pre-fill the pick table; the page also works standalone with its own
 * product search, since the product-list bulk-select entry point doesn't exist yet.
 *
 * Copy modes (§6 "Copies"): a fixed number per pick (built in directly — "نسخة لكل قطعة في
 * المخزون" and "حسب الكمية المستلمة" are just two ways to pre-fill that same fixed number, done by
 * the caller before navigating here with `?qty=`).
 */
import { computed, onBeforeUnmount, reactive, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { Minus, Plus, Printer, Trash2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import PdfPreview from '@/modules/core/components/ui/PdfPreview.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatMoney } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { renderLabelsAndSave, renderLabelsPreview, type PreviewError } from '@/modules/core/services/pdfService';
import { getProducts } from '@/modules/products/services/productService';
import type { Product } from '@/modules/products/types';
import { LABEL_PRESETS, type LabelOptions } from '@/modules/templates/types';

const route = useRoute();
const toast = useToast();

// --- Product picks -----------------------------------------------------------------------------
interface Pick {
  productId: string;
  name: string;
  sku: string;
  barcode?: string;
  price: number;
  copies: number;
}

const picks = reactive<Pick[]>([]);
const allProducts = ref<Product[]>([]);
const search = ref('');
const loadingProducts = ref(true);

async function loadProducts() {
  loadingProducts.value = true;
  try {
    allProducts.value = await getProducts({ includeInactive: false });
    // Pre-fill from query params, e.g. `?productId=p1&qty=12` (a product page's "طباعة ملصقات")
    // or `?productId=p1&productId=p2&qty=5&qty=12` (a purchase receipt's "طباعة ملصقات للكميات
    // المستلمة" — one productId/qty pair per received line, same order, PurchaseReceivePage.vue's
    // `printLabels`).
    const productIds = ([] as string[]).concat(route.query.productId as string | string[] | undefined ?? []);
    const qtys = ([] as string[]).concat(route.query.qty as string | string[] | undefined ?? []);
    productIds.forEach((id, i) => {
      const p = allProducts.value.find((x) => x.id === id);
      if (p) addPick(p, num0(qtys[i] ?? qtys[0]) || 1);
    });
  } catch (err) {
    toast.error(err);
  } finally {
    loadingProducts.value = false;
  }
}
loadProducts();

const searchResults = computed(() => {
  const q = search.value.trim().toLowerCase();
  if (!q) return [];
  return allProducts.value.filter((p) => p.name.toLowerCase().includes(q) || p.sku.toLowerCase().includes(q) || p.barcode?.includes(q)).slice(0, 8);
});

function addPick(product: Product, copies = 1) {
  const existing = picks.find((p) => p.productId === product.id);
  if (existing) {
    existing.copies += copies;
    return;
  }
  picks.push({ productId: product.id, name: product.name, sku: product.sku, barcode: product.barcode, price: product.price, copies });
}

function pickFromSearch(product: Product) {
  addPick(product);
  search.value = '';
}

function removePick(productId: string) {
  const idx = picks.findIndex((p) => p.productId === productId);
  if (idx >= 0) picks.splice(idx, 1);
}

const totalLabels = computed(() => picks.reduce((a, p) => a + Math.max(0, Math.floor(p.copies) || 0), 0));

// --- Label template + options -------------------------------------------------------------------
const presetId = ref(LABEL_PRESETS[0].id);
const options = reactive<LabelOptions>({ ...LABEL_PRESETS[0].options });

watch(presetId, (id) => {
  const preset = LABEL_PRESETS.find((p) => p.id === id);
  if (preset) Object.assign(options, preset.options);
});

const presetOptions = LABEL_PRESETS.map((p) => ({ value: p.id, label: p.name }));

// --- Live preview --------------------------------------------------------------------------------
const previewPages = ref<string[]>([]);
const previewLoading = ref(false);
const previewError = ref('');

function labelPicksForPreview() {
  return picks
    .filter((p) => p.copies > 0)
    .slice(0, 6) // preview only needs the first sheet/label, not the whole batch
    .map((p) => ({ productId: p.productId, name: p.name, sku: p.sku, barcode: p.barcode, priceText: formatMoney(p.price), copies: Math.min(p.copies, 6) }));
}

let debounceHandle: ReturnType<typeof setTimeout> | null = null;
function schedulePreview() {
  if (debounceHandle) clearTimeout(debounceHandle);
  debounceHandle = setTimeout(runPreview, 300);
}
onBeforeUnmount(() => {
  if (debounceHandle) clearTimeout(debounceHandle);
});

async function runPreview() {
  if (picks.length === 0) {
    previewPages.value = [];
    return;
  }
  previewLoading.value = true;
  previewError.value = '';
  try {
    const result = await renderLabelsPreview(labelPicksForPreview(), options);
    previewPages.value = result.pages;
  } catch (err) {
    const diagnostics = (err as PreviewError)?.diagnostics ?? [{ line: null, column: null, severity: 'error', message: String(err) }];
    previewError.value = diagnostics.map((d) => d.message).join('؛ ');
    previewPages.value = [];
  } finally {
    previewLoading.value = false;
  }
}

watch([() => JSON.stringify(picks), () => JSON.stringify(options)], () => schedulePreview());
runPreview();

// --- Print ---------------------------------------------------------------------------------------
const printing = ref(false);
async function printLabels() {
  if (picks.length === 0) {
    toast.error('أضف منتجاً واحداً على الأقل');
    return;
  }
  printing.value = true;
  try {
    const items = picks
      .filter((p) => p.copies > 0)
      .map((p) => ({ productId: p.productId, name: p.name, sku: p.sku, barcode: p.barcode, priceText: formatMoney(p.price), copies: p.copies }));
    const ok = await renderLabelsAndSave(items, options, 'ملصقات.pdf');
    if (ok) toast.success('تم إنشاء ملف الملصقات');
  } catch (err) {
    toast.error(err);
  } finally {
    printing.value = false;
  }
}

</script>

<template>
  <div>
    <PageHeader title="منشئ الملصقات" subtitle="اختر المنتجات وقالب الملصق، ثم اطبع" back="/products">
      <template #actions>
        <AppButton variant="primary" :icon="Printer" :loading="printing" :disabled="totalLabels === 0" @click="printLabels">طباعة ({{ totalLabels }})</AppButton>
      </template>
    </PageHeader>

    <div class="grid items-start gap-5 xl:grid-cols-[1fr_1fr]">
      <div class="space-y-4">
        <AppCard title="المنتجات" padding="sm">
          <div class="space-y-3">
            <div class="relative">
              <SearchInput v-model="search" placeholder="ابحث بالاسم أو الرمز أو الباركود…" />
              <div v-if="searchResults.length" class="absolute z-10 mt-1 w-full rounded-lg border border-border bg-background shadow-lg">
                <button
                  v-for="p in searchResults"
                  :key="p.id"
                  type="button"
                  class="flex w-full items-center justify-between gap-2 px-3 py-2 text-start text-body hover:bg-surface-hover"
                  @click="pickFromSearch(p)"
                >
                  <span>{{ p.name }}</span>
                  <span class="num text-tiny text-text-secondary">{{ p.sku }}</span>
                </button>
              </div>
            </div>

            <p v-if="loadingProducts" class="text-xs text-text-secondary">جارٍ تحميل المنتجات…</p>
            <p v-else-if="picks.length === 0" class="text-xs text-text-secondary">لم تُضف منتجات بعد — ابحث أعلاه لإضافتها.</p>

            <table v-if="picks.length" class="w-full text-body">
              <thead class="text-xs text-text-secondary">
                <tr class="border-b border-border">
                  <th class="px-2 py-2 text-start font-medium">الصنف</th>
                  <th class="px-2 py-2 text-start font-medium">السعر</th>
                  <th class="w-32 px-2 py-2 text-start font-medium">النسخ</th>
                  <th class="w-8"></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="p in picks" :key="p.productId" class="border-b border-border last:border-0">
                  <td class="px-2 py-2">
                    {{ p.name }}
                    <span class="num block text-tiny text-text-secondary">{{ p.sku }}</span>
                  </td>
                  <td class="px-2 py-2"><span class="num">{{ formatMoney(p.price) }}</span></td>
                  <td class="px-2 py-2">
                    <div class="flex items-center gap-1">
                      <button type="button" class="flex size-7 items-center justify-center rounded-md border border-border hover:bg-surface-hover" aria-label="إنقاص" @click="p.copies = Math.max(0, p.copies - 1)"><Minus class="size-3.5" /></button>
                      <input v-model.number="p.copies" type="number" min="0" step="1" class="control num h-8 w-14 text-center" />
                      <button type="button" class="flex size-7 items-center justify-center rounded-md border border-border hover:bg-surface-hover" aria-label="زيادة" @click="p.copies += 1"><Plus class="size-3.5" /></button>
                    </div>
                  </td>
                  <td class="px-2 py-2">
                    <button type="button" class="rounded p-1 text-text-secondary hover:text-danger" aria-label="إزالة" @click="removePick(p.productId)"><Trash2 class="size-4" /></button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </AppCard>

        <AppCard title="قالب الملصق" padding="sm">
          <div class="space-y-3">
            <AppSelect v-model="presetId" label="القالب" :options="presetOptions" />
            <div class="grid grid-cols-2 gap-3">
              <AppInput v-model.number="options.widthMm" type="number" min="10" step="0.5" label="العرض (مم)" />
              <AppInput v-model.number="options.heightMm" type="number" min="10" step="0.5" label="الارتفاع (مم)" />
            </div>
            <div v-if="options.layout === 'sheet'" class="grid grid-cols-3 gap-3">
              <AppInput v-model.number="options.cols" type="number" min="1" label="أعمدة" />
              <AppInput v-model.number="options.rows" type="number" min="1" label="صفوف" />
              <AppInput v-model.number="options.startCell" type="number" min="1" label="بداية من خلية" hint="لإعادة استخدام ورقة مستخدمة جزئياً" />
            </div>
            <div class="grid grid-cols-2 gap-2">
              <AppSwitch v-model="options.showStoreName" label="اسم المتجر" />
              <AppSwitch v-model="options.showPrice" label="السعر" />
              <AppSwitch v-model="options.showSku" label="رمز الصنف" />
              <AppSwitch v-model="options.showBatch" label="التشغيلة والصلاحية" />
              <AppSwitch v-model="options.showBarcode" label="الباركود" />
              <AppSwitch v-model="options.showQr" label="رمز QR" />
            </div>
          </div>
        </AppCard>
      </div>

      <AppCard padding="none" class="h-[70vh] overflow-hidden xl:sticky xl:top-4">
        <PdfPreview :pages="previewPages" :loading="previewLoading" empty-message="أضف منتجاً لعرض معاينة الملصق" />
        <p v-if="previewError" class="border-t border-border bg-danger/10 px-3 py-2 text-xs text-danger">{{ previewError }}</p>
      </AppCard>
    </div>
  </div>
</template>

