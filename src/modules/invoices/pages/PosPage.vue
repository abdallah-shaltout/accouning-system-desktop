<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { isTauri } from '@tauri-apps/api/core';
import {
  CircleCheck,
  Keyboard,
  Minus,
  Moon,
  PackageSearch,
  PauseCircle,
  Percent,
  Plus,
  Printer,
  ScanBarcode,
  ShoppingCart,
  Sun,
  Trash,
  Undo2,
  UserRound,
  Wallet,
  X,
} from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useHotkeys } from '@/modules/core/controllers/useHotkeys';
import { resolvedTheme, toggleTheme } from '@/modules/core/controllers/useTheme';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { dirIcon } from '@/modules/core/helpers/dirIcon';
import { formatNumber, formatTime } from '@/modules/core/helpers/format';
import { matchesSearch } from '@/modules/core/helpers/search';
import * as printService from '@/modules/core/services/printService';
import { on } from '@/mocks/events';
import { getCustomers } from '@/modules/parties/services/partyService';
import type { Customer } from '@/modules/parties/types';
import { useCatalogStore } from '@/modules/products/controllers/useCatalogStore';
import { getBatches } from '@/modules/products/services/inventoryService';
import { getProducts } from '@/modules/products/services/productService';
import type { Product, ProductBatch, ProductUnit } from '@/modules/products/types';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import CashInOutDialog from '../components/CashInOutDialog.vue';
import CloseShiftDialog from '../components/CloseShiftDialog.vue';
import CustomPriceDialog from '../components/CustomPriceDialog.vue';
import DiscountDialog from '../components/DiscountDialog.vue';
import HeldSalesDialog from '../components/HeldSalesDialog.vue';
import OpenShiftDialog from '../components/OpenShiftDialog.vue';
import ReturnByScanDialog from '../components/ReturnByScanDialog.vue';
import ShiftBar from '../components/ShiftBar.vue';
import TenderDialog from '../components/TenderDialog.vue';
import { usePosStore } from '../controllers/usePosStore';
import { changeDue } from '../helpers/totals';
import {
  closePosShift,
  createSale,
  getCurrentShift,
  getHeldSales,
  holdSale,
  discardHeldSale,
  openPosShift,
  recordCashInOut,
  resumeHeldSale,
} from '../services/invoiceService';
import type { HeldSale, Invoice, Tender } from '../types';
import type { ShiftRow } from '../services/invoiceService';

const router = useRouter();
const toast = useToast();
const confirmDialog = useConfirm();
const auth = useAuthStore();
const settings = useSettingsStore();
const catalog = useCatalogStore();
const cart = usePosStore();

const products = ref<Product[]>([]);
const customers = ref<Customer[]>([]);
const loading = ref(true);
const loadError = ref<string | null>(null);
const search = ref('');
const category = ref<string>('all');
const searchInput = ref<HTMLInputElement>();
const customerPicker = ref<InstanceType<typeof AppCombobox>>();
const submitting = ref(false);
const completed = ref<{ invoice: Invoice; change: number } | null>(null);
const shortcutsOpen = ref(false);
const now = ref(new Date());
const clock = setInterval(() => (now.value = new Date()), 30_000);
onBeforeUnmount(() => clearInterval(clock));

// --- Shift (docs/v2/06 §5) ---------------------------------------------------------------------
const shift = ref<ShiftRow | null>(null);
const openShiftDialogOpen = ref(false);
const closeShiftDialogOpen = ref(false);
const cashInOutOpen = ref(false);
const shiftBusy = ref(false);
const requireOpenShift = computed(() => settings.settings?.pos?.requireOpenShift !== false);

async function loadShift() {
  shift.value = (await getCurrentShift(cart.terminalId)) ?? null;
  cart.shiftId = shift.value?.id;
}

async function openShift(openingFloat: number, denominations?: any) {
  shiftBusy.value = true;
  try {
    await openPosShift({ terminalId: cart.terminalId, openingFloat, openingDenominations: denominations });
    await loadShift();
    openShiftDialogOpen.value = false;
    toast.success('تم فتح الوردية');
  } catch (err) {
    toast.error(err);
  } finally {
    shiftBusy.value = false;
  }
}

async function closeShiftNow(countedCash: number, denominations: any, handoverMode: 'HANDOVER' | 'DROP', note?: string) {
  if (!shift.value) return;
  if (held.value.length) {
    const ok = await confirmDialog({ title: 'توجد مبيعات معلّقة', message: 'سيتم إغلاق الوردية رغم وجود مبيعات معلّقة — هل تريد المتابعة؟', confirmText: 'إغلاق على أي حال', danger: true });
    if (!ok) return;
  }
  shiftBusy.value = true;
  try {
    await closePosShift(shift.value.id, { countedCash, closingDenominations: denominations, handoverMode, note });
    closeShiftDialogOpen.value = false;
    toast.success('تم إغلاق الوردية', 'تم ترحيل الفرق للحساب المناسب');
    router.push('/pos/shifts');
  } catch (err) {
    toast.error(err);
  } finally {
    shiftBusy.value = false;
  }
}

async function cashInOut(kind: 'PAY_IN' | 'PAY_OUT' | 'BANK_DROP', amount: number, note?: string) {
  shiftBusy.value = true;
  try {
    await recordCashInOut(cart.terminalId, kind, amount, note);
    await loadShift();
    cashInOutOpen.value = false;
    toast.success('تم التسجيل');
  } catch (err) {
    toast.error(err);
  } finally {
    shiftBusy.value = false;
  }
}

// --- Held sales (F6) -----------------------------------------------------------------------------
const held = ref<HeldSale[]>([]);
const heldDialogOpen = ref(false);

async function loadHeld() {
  held.value = await getHeldSales(cart.terminalId);
}

async function holdCurrentSale() {
  if (cart.isEmpty) return;
  try {
    await holdSale(cart.toHeld(undefined));
    cart.clear();
    await loadHeld();
    toast.info('تم تعليق البيع', 'اضغط F6 لاستئنافه لاحقاً');
  } catch (err) {
    toast.error(err);
  }
}

async function resumeHeld(id: string) {
  if (!cart.isEmpty) {
    const ok = await confirmDialog({ title: 'استئناف بيع معلّق؟', message: 'سيتم إفراغ السلة الحالية.', confirmText: 'متابعة', danger: true });
    if (!ok) return;
  }
  try {
    const heldSale = await resumeHeldSale(id);
    cart.restoreHeld(heldSale, products.value);
    heldDialogOpen.value = false;
  } catch (err) {
    toast.error(err);
  }
}

async function discardHeld(id: string) {
  await discardHeldSale(id);
  await loadHeld();
}

// --- Return by scan (F7) --------------------------------------------------------------------------
const returnDialogOpen = ref(false);

// --- Load -----------------------------------------------------------------------------------------
async function load() {
  loading.value = true;
  loadError.value = null;
  try {
    const [p, c] = await Promise.all([getProducts(), getCustomers(), catalog.load(), loadShift(), loadHeld()]);
    products.value = p;
    customers.value = c;
  } catch (err) {
    loadError.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}
onMounted(async () => {
  await load();
  searchInput.value?.focus();
});

/**
 * v2 §7 "Speed": barcode index `Map<barcode, {productId, unitId}>`, built once and rebuilt only when
 * the catalog changes — never on every keystroke. Covers the product's own `barcode` plus every
 * unit's `barcodes[]` (Phase 6, docs/v2/06 §1 "scanning a unit's barcode selects that unit").
 */
const barcodeIndex = ref(new Map<string, { productId: string; unitId?: string }>());
function rebuildBarcodeIndex() {
  const index = new Map<string, { productId: string; unitId?: string }>();
  for (const p of products.value) {
    if (p.barcode) index.set(p.barcode, { productId: p.id });
    for (const u of p.units ?? []) {
      for (const code of u.barcodes) index.set(code, { productId: p.id, unitId: u.factor === 1 ? undefined : u.id });
    }
  }
  barcodeIndex.value = index;
}
watch(products, rebuildBarcodeIndex, { immediate: true });
const unsubCatalog = on('catalog:changed', async () => {
  products.value = await getProducts();
  rebuildBarcodeIndex();
});
onBeforeUnmount(unsubCatalog);

// --- pricing: the cashier's price list, falling back to the base price ---
const priceList = computed(() => catalog.priceLists.find((p) => p.id === auth.user?.priceListId && p.active));
function priceOf(p: Product, unit?: ProductUnit): number {
  if (unit) {
    const unitPrice = p.unitPrices?.find((x) => x.unitId === unit.id && x.priceListId === priceList.value?.id);
    return unitPrice?.value ?? unit.price;
  }
  return (priceList.value && p.prices?.find((x) => x.priceListId === priceList.value!.id)?.value) ?? p.price;
}

const maxDiscount = computed(() => auth.user?.maxDiscount ?? 0);
const categories = computed(() => [{ id: 'all', name: 'الكل' }, ...catalog.categories.filter((c) => c.productCount > 0)]);

const visible = computed(() =>
  products.value.filter(
    (p) => (category.value === 'all' || p.categoryId === category.value) && matchesSearch([p.name, p.sku, p.barcode], search.value),
  ),
);

function remaining(p: Product) {
  return p.type === 'service' ? Infinity : p.stockQty - cart.qtyInCart(p.id);
}

// --- Unit picker (shown when a product has more than one active unit) ---
const unitPickerProduct = ref<Product | null>(null);
const unitPickerQty = ref(1);
const unitPickerFocusQty = ref(false);

/**
 * Cart qty inputs by line id. A product-card click lands the cursor in the added line's qty
 * (selected, so typing replaces it); Enter/Esc there returns to the search box. Scans don't do
 * this — the cursor stays in the search box for the next scan.
 */
const qtyInputs = new Map<string, HTMLInputElement>();
function setQtyRef(lineId: string, el: unknown) {
  if (el) qtyInputs.set(lineId, el as HTMLInputElement);
  else qtyInputs.delete(lineId);
}
function focusActiveQty() {
  nextTick(() => {
    const el = cart.activeId ? qtyInputs.get(cart.activeId) : undefined;
    el?.focus();
    el?.select();
  });
}
function onQtyChange(lineId: string, e: Event) {
  const input = e.target as HTMLInputElement;
  changeQty(lineId, Number(input.value));
  // A rejected qty (over stock) leaves the line untouched — show its real qty again.
  const line = cart.lines.find((l) => l.id === lineId);
  if (line) input.value = String(line.qty);
}
function onQtyEscape(lineId: string, e: Event) {
  const line = cart.lines.find((l) => l.id === lineId);
  if (line) (e.target as HTMLInputElement).value = String(line.qty);
  searchInput.value?.focus();
}

async function addProductWithUnit(p: Product, unit: ProductUnit | undefined, qty: number, focusQty = false) {
  const price = priceOf(p, unit);
  let batchId: string | undefined;
  let batchNo: string | undefined;
  if (p.trackBatches) {
    const batches = await getBatches(p.id);
    const usable = batches.filter((b) => !b.expiryDate || b.expiryDate >= new Date().toISOString().slice(0, 10));
    if (!usable.length && batches.length) {
      toast.warning('كل التشغيلات منتهية الصلاحية', p.name);
      return;
    }
    if (usable[0]) {
      batchId = usable[0].id;
      batchNo = usable[0].batchNo;
    }
  }
  if (!cart.add(p, { price, listPrice: price, qty, unit, batchId, batchNo })) {
    toast.warning('الكمية غير متوفرة', `المتوفر من "${p.name}": ${formatNumber(p.stockQty)}`);
  } else if (focusQty) focusActiveQty();
}

/** `focusQty`: product-card clicks move the cursor to the new line's qty (see `qtyInputs`). */
function addProduct(p: Product, focusQty = false) {
  const qty = cart.takeQtyPrefix();
  const activeUnits = (p.units ?? []).filter((u) => u.active && u.defaultForSale !== false);
  if (activeUnits.length > 1) {
    unitPickerProduct.value = p;
    unitPickerQty.value = qty;
    unitPickerFocusQty.value = focusQty;
    return;
  }
  void addProductWithUnit(p, activeUnits[0], qty, focusQty);
}

/** Enter / scan in the search box: exact barcode/SKU wins (via the index, <50ms), then a single visible match. */
function onSearchEnter() {
  const raw = search.value.trim();
  if (!raw) return;
  // `n*` qty-multiplier prefix (docs/v2/06 §1 "3* then scan = qty 3").
  const multiplier = raw.match(/^(\d+)\*(.*)$/);
  if (multiplier) {
    cart.qtyPrefix = Number(multiplier[1]);
    const rest = multiplier[2].trim();
    if (!rest) {
      search.value = '';
      return;
    }
    search.value = rest;
  }
  const code = search.value.trim();
  const hit = barcodeIndex.value.get(code);
  if (hit) {
    const product = products.value.find((p) => p.id === hit.productId);
    if (product) {
      const unit = hit.unitId ? product.units?.find((u) => u.id === hit.unitId) : undefined;
      void addProductWithUnit(product, unit, cart.takeQtyPrefix());
      search.value = '';
      return;
    }
  }
  const exact = products.value.find((p) => p.barcode === code || p.sku.toLowerCase() === code.toLowerCase());
  const target = exact ?? (visible.value.length === 1 ? visible.value[0] : undefined);
  if (target) {
    addProduct(target);
    search.value = '';
  } else if (!visible.value.length) toast.warning('لا يوجد منتج بهذا الرمز', code);
}

function changeQty(lineId: string, qty: number) {
  if (!cart.setQty(lineId, qty)) toast.warning('وصلت للحد المتوفر في المخزون');
}

// --- Discount (F8 line / Shift+F8 invoice) -------------------------------------------------------
const discountDialogOpen = ref(false);
const discountKind = ref<'line' | 'invoice'>('line');

function openLineDiscount() {
  if (!cart.activeLine) return;
  discountKind.value = 'line';
  discountDialogOpen.value = true;
}
function openInvoiceDiscount() {
  if (cart.isEmpty) return;
  discountKind.value = 'invoice';
  discountDialogOpen.value = true;
}
function applyDiscount(value: number, isPct: boolean, _approvedBy?: string) {
  if (discountKind.value === 'line' && cart.activeLine) {
    cart.setLineDiscount(cart.activeLine.id, value, isPct);
  } else if (discountKind.value === 'invoice') {
    cart.discountIsAmount = !isPct;
    if (isPct) cart.discountRate = value;
    else cart.discountAmount = value;
  }
}

// --- Custom price ----------------------------------------------------------------------------------
const priceDialogOpen = ref(false);
const priceDialogBatches = ref<ProductBatch[]>([]);
async function openPriceDialog() {
  if (!cart.activeLine || settings.settings?.pos?.overridePrice === false) return;
  const product = products.value.find((p) => p.id === cart.activeLine!.productId);
  priceDialogBatches.value = product?.trackBatches ? await getBatches(product.id) : [];
  priceDialogOpen.value = true;
}
function priceFloor(): number {
  const line = cart.activeLine;
  if (!line) return 0;
  const product = products.value.find((p) => p.id === line.productId);
  if (!product) return 0;
  return Math.max(product.minPrice ?? 0, settings.settings?.pos?.sellBelowCost ? 0 : product.costPrice);
}
function applyCustomPrice(patch: { price: number; reason?: string; batchId?: string }) {
  const line = cart.activeLine;
  if (!line) return;
  line.price = patch.price;
  line.priceOverrideReason = patch.reason;
  if (patch.batchId) {
    line.batchId = patch.batchId;
    const product = products.value.find((p) => p.id === line.productId);
    line.batchNo = undefined; // resolved server-side from batchId; the label isn't needed client-side after this
    void product;
  }
}

const customerOptions = computed(() =>
  customers.value.map((c) => ({ value: c.id, label: c.name, sublabel: [c.phone, c.balance > 0 ? `مستحق ${c.balance.toFixed(2)}` : ''].filter(Boolean).join(' · '), keywords: `${c.phone ?? ''} ${c.vatNumber ?? ''}` })),
);
const customer = computed(() => customers.value.find((c) => c.id === cart.customerId));

const draft = computed(() => ({
  customerId: cart.customerId,
  discountRate: cart.discountIsAmount ? 0 : cart.discountRate,
  discountAmount: cart.discountIsAmount ? cart.discountAmount : undefined,
  note: cart.note || undefined,
  lines: cart.toSaleLines(),
  source: 'POS' as const,
  shiftId: cart.shiftId,
}));

const tenderOpen = ref(false);

function openCheckout() {
  if (cart.isEmpty) {
    toast.info('السلة فارغة', 'أضف منتجاً أولاً');
    return;
  }
  if (requireOpenShift.value && !shift.value) {
    toast.warning('يجب فتح وردية للبيع', 'اضغط على "فتح وردية" أولاً');
    openShiftDialogOpen.value = true;
    return;
  }
  tenderOpen.value = true;
}

async function checkout(payment: { tenders: Tender[]; paidAmount: number; tenderedAmount?: number }) {
  submitting.value = true;
  const checkoutStart = performance.now();
  try {
    const invoice = await createSale({ ...draft.value, paymentMethod: 'cash', ...payment });
    tenderOpen.value = false;
    completed.value = { invoice, change: payment.tenderedAmount ? changeDue(invoice.grandTotal, payment.tenderedAmount) : 0 };
    cart.clear();
    products.value = await getProducts(); // stock changed
    customers.value = await getCustomers();
    await loadShift();
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.debug(`[pos] checkout round-trip: ${(performance.now() - checkoutStart).toFixed(1)}ms`);
    }
  } catch (err) {
    toast.error(err, 'تعذر إتمام البيع');
  } finally {
    submitting.value = false;
  }
}

async function newSale() {
  if (completed.value) {
    completed.value = null;
  } else if (!cart.isEmpty) {
    const ok = await confirmDialog({ title: 'بدء بيع جديد؟', message: 'سيتم إفراغ السلة الحالية.', confirmText: 'إفراغ السلة', danger: true });
    if (!ok) return;
    cart.clear();
  }
  search.value = '';
  searchInput.value?.focus();
}

/**
 * Phase 14 (docs/v2/12-documents-pdf-excel.md §5): in the desktop app with a
 * thermal printer configured, prints natively (async — never blocks the next
 * sale, per §5) instead of opening the browser print-preview route. Falls
 * back to the existing `/print/invoices/:id` browser route in dev mode, or
 * when the store is still set to A4/no thermal config yet.
 */
async function printReceipt(invoiceId?: string) {
  const id = invoiceId ?? completed.value?.invoice.id;
  if (!id) return;
  const printStart = performance.now();
  const printerSettings = settings.settings?.printer;
  if (isTauri() && printerSettings?.mode === 'thermal') {
    const outcome = await printService.printReceipt(id, completed.value?.invoice.paymentMethod === 'cash');
    if (outcome.ok) {
      if (import.meta.env.DEV) console.debug(`[pos] checkout→receipt (native, spawn only): ${(performance.now() - printStart).toFixed(1)}ms`);
      return;
    }
  }
  router.push({ path: `/print/invoices/${id}`, query: { auto: '1', back: '/pos' } });
}

const anyModal = computed(
  () =>
    tenderOpen.value ||
    !!completed.value ||
    shortcutsOpen.value ||
    heldDialogOpen.value ||
    returnDialogOpen.value ||
    discountDialogOpen.value ||
    priceDialogOpen.value ||
    openShiftDialogOpen.value ||
    closeShiftDialogOpen.value ||
    cashInOutOpen.value ||
    !!unitPickerProduct.value,
);

/** Run `fn` only when no dialog is open; returning false lets the key keep its default behavior. */
const whenIdle = (fn: () => unknown) => () => {
  if (anyModal.value) return false;
  fn();
};
function bumpActive(delta: number) {
  if (anyModal.value || !cart.activeId) return false;
  const line = cart.activeLine;
  if (!line) return false;
  changeQty(line.id, line.qty + delta);
}

useHotkeys({
  F1: () => void (shortcutsOpen.value = !shortcutsOpen.value),
  F2: { id: 'pos.search', label: 'البحث / الباركود', group: 'نقطة البيع', handler: whenIdle(() => searchInput.value?.focus()) },
  F4: { id: 'pos.pickCustomer', label: 'اختيار العميل', group: 'نقطة البيع', handler: whenIdle(() => customerPicker.value?.open()) },
  F6: {
    id: 'pos.holdResume',
    label: 'تعليق / استئناف البيع',
    group: 'نقطة البيع',
    handler: whenIdle(() => (cart.isEmpty ? (heldDialogOpen.value = true) : void holdCurrentSale())),
  },
  F7: { id: 'pos.return', label: 'إرجاع', group: 'نقطة البيع', handler: whenIdle(() => (returnDialogOpen.value = true)) },
  F8: { id: 'pos.lineDiscount', label: 'خصم الصنف', group: 'نقطة البيع', handler: whenIdle(openLineDiscount) },
  'shift+F8': { id: 'pos.invoiceDiscount', label: 'خصم الفاتورة', group: 'نقطة البيع', handler: whenIdle(openInvoiceDiscount) },
  F9: { id: 'pos.newSale', label: 'بيع جديد', group: 'نقطة البيع', handler: () => void newSale() },
  F10: { id: 'pos.cashInOut', label: 'إيداع/سحب نقدي', group: 'نقطة البيع', handler: whenIdle(() => (cashInOutOpen.value = true)) },
  F12: { id: 'pos.checkout', label: 'الدفع', group: 'نقطة البيع', handler: whenIdle(openCheckout) },
  'ctrl+Enter': { id: 'pos.checkout', label: 'الدفع', group: 'نقطة البيع', handler: whenIdle(openCheckout) },
  'ctrl+p': { id: 'pos.reprintReceipt', label: 'إعادة طباعة الإيصال', group: 'نقطة البيع', handler: () => (completed.value ? (void printReceipt(), true) : false) },
  '+': () => bumpActive(1),
  '-': () => bumpActive(-1),
  Enter: () => (completed.value ? void newSale() : false),
});

const shortcuts = [
  ['F2', 'البحث / الباركود'],
  ['F4', 'اختيار العميل'],
  ['F6', 'تعليق / استئناف البيع'],
  ['F7', 'إرجاع'],
  ['F8', 'خصم الصنف'],
  ['Shift+F8', 'خصم الفاتورة'],
  ['F9', 'بيع جديد'],
  ['F10', 'إيداع/سحب نقدي'],
  ['F12', 'الدفع'],
  ['Ctrl+P', 'إعادة طباعة الإيصال'],
  ['+ / −', 'كمية الصنف المحدد'],
  ['Enter / Esc', 'من خانة الكمية: تأكيد / تراجع والعودة للبحث'],
  ['n*', 'مضاعف الكمية قبل المسح، مثال 3*'],
  ['Enter', 'تأكيد الدفع / بيع جديد'],
  ['F1', 'عرض الاختصارات'],
];
</script>

<template>
  <div class="flex h-screen flex-col overflow-hidden bg-background">
    <!-- Top bar -->
    <header class="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-border px-4">
      <div class="flex items-center gap-3">
        <AppButton size="sm" variant="ghost" :icon="dirIcon.back" icon-rtl-flip to="/">لوحة التحكم</AppButton>
        <div class="h-5 w-px bg-border" />
        <span class="text-body font-semibold">نقطة البيع</span>
        <span class="text-body text-text-secondary">{{ settings.settings?.storeName }}</span>
      </div>
      <div class="flex items-center gap-3 text-body text-text-secondary">
        <ShiftBar :shift="shift" :now="now" />
        <AppButton v-if="!shift" size="sm" variant="primary" :icon="Wallet" @click="openShiftDialogOpen = true">فتح وردية</AppButton>
        <template v-else>
          <AppButton size="sm" variant="ghost" kbd="F10" @click="cashInOutOpen = true">إيداع/سحب</AppButton>
          <AppButton size="sm" variant="ghost" @click="closeShiftDialogOpen = true">إغلاق الوردية</AppButton>
        </template>
        <button
          type="button"
          class="relative rounded-md p-1.5 hover:bg-surface-hover hover:text-text-primary"
          aria-label="مبيعات معلّقة"
          @click="heldDialogOpen = true"
        >
          <PauseCircle class="size-4" />
          <span v-if="held.length" class="num absolute -end-1 -top-1 flex size-4 items-center justify-center rounded-full bg-primary text-tiny text-on-primary">{{ held.length }}</span>
        </button>
        <button type="button" class="rounded-md p-1.5 hover:bg-surface-hover hover:text-text-primary" aria-label="إرجاع" @click="returnDialogOpen = true">
          <Undo2 class="size-4" />
        </button>
        <span v-if="priceList" class="rounded-full bg-primary/10 px-2 py-0.5 text-xs text-primary">{{ priceList.name }}</span>
        <span class="flex items-center gap-1.5"><UserRound class="size-3.5" />{{ auth.user?.name }}</span>
        <span class="num">{{ formatTime(now.toISOString()) }}</span>
        <button type="button" class="rounded-md p-1.5 hover:bg-surface-hover hover:text-text-primary" aria-label="اختصارات لوحة المفاتيح" @click="shortcutsOpen = true">
          <Keyboard class="size-4" />
        </button>
        <button type="button" class="rounded-md p-1.5 hover:bg-surface-hover hover:text-text-primary" aria-label="تبديل المظهر" @click="toggleTheme">
          <Sun v-if="resolvedTheme === 'dark'" class="size-4" />
          <Moon v-else class="size-4" />
        </button>
      </div>
    </header>

    <div class="flex min-h-0 flex-1">
      <!-- Products (start / right side) -->
      <section class="flex min-w-0 flex-1 flex-col">
        <div class="space-y-3 border-b border-border p-4">
          <div class="relative">
            <ScanBarcode class="pointer-events-none absolute start-3 top-1/2 size-5 -translate-y-1/2 text-text-secondary" />
            <input
              ref="searchInput"
              v-model="search"
              class="control h-11 ps-11 text-lead"
              placeholder="امسح الباركود أو ابحث باسم المنتج… (3* + مسح = كمية 3)"
              @keydown.enter.prevent="onSearchEnter"
              @keydown.esc="search = ''"
            />
            <kbd class="num pointer-events-none absolute end-3 top-1/2 -translate-y-1/2 rounded border border-border px-1.5 text-tiny text-text-secondary">F2</kbd>
          </div>
          <p v-if="cart.qtyPrefix" class="text-xs text-primary">الكمية القادمة: <span class="num">{{ cart.qtyPrefix }}</span></p>
          <div class="flex gap-1.5 overflow-x-auto pb-0.5">
            <button
              v-for="c in categories"
              :key="c.id"
              type="button"
              class="h-8 shrink-0 rounded-full border px-3.5 text-body transition-colors"
              :class="category === c.id ? 'border-primary bg-primary text-on-primary' : 'border-border text-text-secondary hover:bg-surface-hover hover:text-text-primary'"
              @click="category = c.id"
            >
              {{ c.name }}
            </button>
          </div>
        </div>

        <div class="flex-1 overflow-y-auto p-4">
          <ErrorState v-if="loadError" :message="loadError" @retry="load" />
          <div v-else-if="loading" class="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-3">
            <div v-for="i in 12" :key="i" class="h-28 animate-shimmer rounded-xl bg-surface" />
          </div>
          <EmptyState v-else-if="!visible.length" :icon="PackageSearch" title="لا توجد منتجات مطابقة" description="جرّب اسماً آخر أو امسح الباركود" />
          <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-3">
            <button
              v-for="p in visible"
              :key="p.id"
              type="button"
              class="group flex h-28 flex-col justify-between rounded-xl border border-border bg-surface p-3 text-start transition-colors hover:border-primary/60 hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-45"
              :disabled="remaining(p) <= 0"
              @click="addProduct(p, true)"
            >
              <span class="line-clamp-2 text-body font-medium leading-snug">{{ p.name }}</span>
              <span class="flex items-end justify-between gap-2">
                <span class="text-lead font-semibold"><MoneyText :value="priceOf(p)" /></span>
                <span
                  class="num text-tiny"
                  :class="p.type === 'service' ? 'text-text-secondary' : remaining(p) <= 0 ? 'text-danger' : remaining(p) <= (p.minStock ?? 0) ? 'text-warning' : 'text-text-secondary'"
                >
                  {{ p.type === 'service' ? 'خدمة' : remaining(p) <= 0 ? 'نفد' : formatNumber(remaining(p)) }}
                </span>
              </span>
            </button>
          </div>
        </div>
      </section>

      <!-- Cart (trailing edge = left in RTL) -->
      <aside class="flex w-[420px] shrink-0 flex-col border-s border-border bg-surface">
        <div class="border-b border-border p-3">
          <div class="flex items-end gap-2">
            <AppCombobox
              ref="customerPicker"
              v-model="cart.customerId"
              class="flex-1"
              :options="customerOptions"
              placeholder="عميل نقدي (بدون اسم)"
              search-placeholder="اسم أو جوال العميل"
              clearable
            />
            <kbd class="num mb-2 rounded border border-border px-1.5 text-tiny text-text-secondary">F4</kbd>
          </div>
          <p v-if="customer && customer.balance > 0" class="mt-1.5 text-xs text-warning">رصيد مستحق سابق: <MoneyText :value="customer.balance" /></p>
        </div>

        <div class="min-h-0 flex-1 overflow-y-auto">
          <EmptyState v-if="cart.isEmpty" :icon="ShoppingCart" title="السلة فارغة" description="اضغط على منتج أو امسح الباركود لإضافته" />
          <ul v-else class="divide-y divide-border" data-testid="pos-cart">
            <li
              v-for="l in cart.lines"
              :key="l.id"
              class="px-3 py-2.5 transition-colors"
              :class="cart.activeId === l.id && 'bg-background'"
              data-testid="pos-cart-line"
              @click="cart.activeId = l.id"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <p class="truncate text-body font-medium">
                    {{ l.name }}
                    <span v-if="l.unit" class="text-xs text-text-secondary">({{ catalog.unitName(l.unit.unitId) }})</span>
                  </p>
                  <p class="text-xs text-text-secondary">
                    <MoneyText :value="l.price" plain /> × <span class="num">{{ formatNumber(l.qty) }}</span>
                    <span v-if="l.price !== l.listPrice" class="ms-1.5 rounded bg-warning/15 px-1 py-0.5 text-tiny text-warning">سعر معدّل</span>
                    <span v-if="l.discount" class="ms-1.5 rounded bg-primary/15 px-1 py-0.5 text-tiny text-primary">خصم {{ l.discountIsPct ? `${l.discount}%` : formatNumber(l.discount) }}</span>
                  </p>
                </div>
                <div class="flex shrink-0 items-center gap-1">
                  <span class="text-body font-medium"><MoneyText :value="l.price * l.qty" plain /></span>
                  <button type="button" class="rounded p-1 text-text-secondary hover:bg-danger/10 hover:text-danger" aria-label="حذف" @click.stop="cart.remove(l.id)">
                    <X class="size-3.5" />
                  </button>
                </div>
              </div>
              <div class="mt-1.5 flex items-center gap-1">
                <button type="button" class="flex size-7 items-center justify-center rounded-md border border-border hover:bg-surface-hover" aria-label="إنقاص" @click.stop="changeQty(l.id, l.qty - 1)">
                  <Minus class="size-3.5" />
                </button>
                <input
                  :ref="(el) => setQtyRef(l.id, el)"
                  :value="l.qty"
                  type="number"
                  min="1"
                  class="control h-7 w-14 px-1 text-center"
                  aria-label="الكمية"
                  data-testid="pos-line-qty"
                  @change="onQtyChange(l.id, $event)"
                  @focus="cart.activeId = l.id"
                  @keydown.enter.prevent="searchInput?.focus()"
                  @keydown.esc.prevent="onQtyEscape(l.id, $event)"
                  @click.stop
                />
                <button type="button" class="flex size-7 items-center justify-center rounded-md border border-border hover:bg-surface-hover" aria-label="زيادة" @click.stop="changeQty(l.id, l.qty + 1)">
                  <Plus class="size-3.5" />
                </button>
                <button type="button" class="ms-2 flex h-7 items-center gap-1 rounded-md border border-border px-2 text-xs hover:bg-surface-hover" @click.stop="(cart.activeId = l.id, openPriceDialog())">
                  السعر
                </button>
                <button type="button" class="flex h-7 items-center gap-1 rounded-md border border-border px-2 text-xs hover:bg-surface-hover" @click.stop="(cart.activeId = l.id, openLineDiscount())">
                  <Percent class="size-3" />خصم
                </button>
              </div>
            </li>
          </ul>
        </div>

        <div class="border-t border-border p-3">
          <button type="button" class="mb-2.5 flex w-full items-center justify-between text-body" @click="openInvoiceDiscount">
            <span class="text-text-secondary">خصم الفاتورة <kbd class="num rounded border border-border px-1 text-tiny">Shift+F8</kbd></span>
            <span v-if="cart.totals.invoiceDiscountAmount > 0" class="text-danger">−<MoneyText :value="cart.totals.invoiceDiscountAmount" plain /></span>
            <span v-else class="text-text-secondary">—</span>
          </button>
          <dl class="space-y-1 text-body">
            <div class="flex justify-between"><dt class="text-text-secondary">المجموع (<span class="num">{{ formatNumber(cart.itemCount) }}</span> قطعة)</dt><dd><MoneyText :value="cart.totals.subTotalAfterLineDiscounts" plain /></dd></div>
            <div class="flex justify-between"><dt class="text-text-secondary">ضريبة القيمة المضافة</dt><dd><MoneyText :value="cart.totals.vat" plain /></dd></div>
            <div class="flex items-baseline justify-between border-t border-border pt-2">
              <dt class="font-medium">الإجمالي</dt>
              <dd class="text-2xl font-semibold tracking-tight"><MoneyText :value="cart.totals.gross" /></dd>
            </div>
          </dl>
          <div class="mt-3 flex gap-2">
            <AppButton size="lg" :icon="Trash" :disabled="cart.isEmpty" kbd="F9" @click="newSale">إلغاء</AppButton>
            <AppButton size="lg" variant="primary" class="flex-1" :disabled="cart.isEmpty" kbd="F12" @click="openCheckout">الدفع</AppButton>
          </div>
        </div>
      </aside>
    </div>

    <TenderDialog
      v-model:open="tenderOpen"
      :total="cart.totals.gross"
      :customer-name="customer?.name"
      :has-customer="!!cart.customerId"
      :draft="draft"
      :submitting="submitting"
      @confirm="checkout"
    />

    <AppModal :open="!!completed" size="sm" @update:open="(v) => !v && newSale()">
      <div v-if="completed" class="py-2 text-center">
        <CircleCheck class="mx-auto size-12 text-success" :stroke-width="1.5" />
        <p class="mt-3 text-lg font-semibold">تم البيع بنجاح</p>
        <p class="num mt-0.5 text-body text-text-secondary">{{ completed.invoice.number }}</p>
        <div class="mt-5 grid grid-cols-2 gap-3 text-start">
          <div class="rounded-lg border border-border bg-surface p-3">
            <p class="text-xs text-text-secondary">الإجمالي</p>
            <p class="mt-0.5 text-lg font-semibold"><MoneyText :value="completed.invoice.grandTotal" /></p>
          </div>
          <div class="rounded-lg border border-success/40 bg-success/10 p-3">
            <p class="text-xs text-text-secondary">الباقي للعميل</p>
            <p class="mt-0.5 text-lg font-semibold text-success"><MoneyText :value="completed.change" /></p>
          </div>
        </div>
        <p v-if="completed.invoice.paidAmount < completed.invoice.grandTotal" class="mt-3 text-xs text-warning">
          المتبقي على حساب العميل: <MoneyText :value="completed.invoice.grandTotal - completed.invoice.paidAmount" />
        </p>
      </div>
      <template #footer>
        <AppButton :icon="Printer" kbd="Ctrl+P" @click="() => printReceipt()">طباعة الإيصال</AppButton>
        <AppButton variant="primary" kbd="Enter" @click="newSale">بيع جديد</AppButton>
      </template>
    </AppModal>

    <!-- Unit picker (docs/v2/06 §1 "shown when the product has more than one unit"). -->
    <AppModal :open="!!unitPickerProduct" title="اختر الوحدة" size="sm" @update:open="(v) => !v && (unitPickerProduct = null)">
      <ul v-if="unitPickerProduct" class="space-y-1.5">
        <li v-for="u in unitPickerProduct.units?.filter((x) => x.active)" :key="u.id">
          <button
            type="button"
            class="flex w-full items-center justify-between rounded-lg border border-border px-3 py-2.5 hover:border-primary/60 hover:bg-surface-hover"
            data-testid="unit-picker-option"
            @click="(void addProductWithUnit(unitPickerProduct!, u, unitPickerQty, unitPickerFocusQty), (unitPickerProduct = null))"
          >
            <span>{{ catalog.unitName(u.unitId) }}</span>
            <MoneyText :value="priceOf(unitPickerProduct, u)" plain />
          </button>
        </li>
      </ul>
    </AppModal>

    <DiscountDialog
      v-model:open="discountDialogOpen"
      :kind="discountKind"
      :max-pct="maxDiscount"
      :initial-value="discountKind === 'line' ? cart.activeLine?.discount : cart.discountIsAmount ? cart.discountAmount : cart.discountRate"
      :initial-is-pct="discountKind === 'line' ? cart.activeLine?.discountIsPct : !cart.discountIsAmount"
      @apply="applyDiscount"
    />
    <CustomPriceDialog
      v-model:open="priceDialogOpen"
      :line="cart.activeLine"
      :floor="priceFloor()"
      :sell-below-cost-allowed="settings.settings?.pos?.sellBelowCost !== false"
      :batches="priceDialogBatches"
      @apply="applyCustomPrice"
    />
    <HeldSalesDialog v-model:open="heldDialogOpen" :held="held" @resume="resumeHeld" @discard="discardHeld" />
    <ReturnByScanDialog v-model:open="returnDialogOpen" :allow-without-receipt="settings.settings?.sales?.refundWithoutReceipt === true" @done="load" />
    <OpenShiftDialog v-model:open="openShiftDialogOpen" :submitting="shiftBusy" @confirm="openShift" />
    <CloseShiftDialog v-model:open="closeShiftDialogOpen" :shift="shift" :submitting="shiftBusy" @confirm="closeShiftNow" />
    <CashInOutDialog v-model:open="cashInOutOpen" :submitting="shiftBusy" @confirm="cashInOut" />

    <AppModal v-model:open="shortcutsOpen" title="اختصارات لوحة المفاتيح" size="sm">
      <ul class="divide-y divide-border text-body">
        <li v-for="[key, label] in shortcuts" :key="key" class="flex items-center justify-between py-2">
          <span>{{ label }}</span>
          <kbd class="num rounded border border-border bg-surface px-2 py-0.5 text-xs">{{ key }}</kbd>
        </li>
      </ul>
    </AppModal>
  </div>
</template>
