<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import {
  ArrowRight,
  CircleCheck,
  Keyboard,
  Minus,
  Moon,
  PackageSearch,
  Plus,
  Printer,
  ScanBarcode,
  ShoppingCart,
  Sun,
  Trash,
  UserRound,
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
import { formatNumber, formatTime } from '@/modules/core/helpers/format';
import { getCustomers } from '@/modules/parties/services/partyService';
import type { Customer } from '@/modules/parties/types';
import { useCatalogStore } from '@/modules/products/controllers/useCatalogStore';
import { getProducts } from '@/modules/products/services/productService';
import type { Product } from '@/modules/products/types';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import CheckoutModal from '../components/CheckoutModal.vue';
import { usePosStore } from '../controllers/usePosStore';
import { changeDue } from '../helpers/totals';
import { createSale } from '../services/invoiceService';
import type { Invoice, SalePaymentMethod } from '../types';

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
const discountInput = ref<HTMLInputElement>();
const checkoutOpen = ref(false);
const submitting = ref(false);
const completed = ref<{ invoice: Invoice; change: number } | null>(null);
const shortcutsOpen = ref(false);
const now = ref(new Date());
const clock = setInterval(() => (now.value = new Date()), 30_000);
onBeforeUnmount(() => clearInterval(clock));

async function load() {
  loading.value = true;
  loadError.value = null;
  try {
    const [p, c] = await Promise.all([getProducts(), getCustomers(), catalog.load()]);
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

// --- pricing: the cashier's price list, falling back to the base price ---
const priceList = computed(() => catalog.priceLists.find((p) => p.id === auth.user?.priceListId && p.active));
function priceOf(p: Product): number {
  return (priceList.value && p.prices?.find((x) => x.priceListId === priceList.value!.id)?.value) ?? p.price;
}

const maxDiscount = computed(() => auth.user?.maxDiscount ?? 0);
const categories = computed(() => [{ id: 'all', name: 'الكل' }, ...catalog.categories.filter((c) => c.productCount > 0)]);

const visible = computed(() => {
  const q = search.value.trim().toLowerCase();
  return products.value.filter(
    (p) => (category.value === 'all' || p.categoryId === category.value) && (!q || `${p.name} ${p.sku} ${p.barcode ?? ''}`.toLowerCase().includes(q)),
  );
});

function remaining(p: Product) {
  return p.type === 'service' ? Infinity : p.stockQty - cart.qtyInCart(p.id);
}

function addProduct(p: Product) {
  if (!cart.add(p, priceOf(p))) toast.warning('الكمية غير متوفرة', `المتوفر من "${p.name}": ${formatNumber(p.stockQty)}`);
}

/** Enter in the search box: exact barcode/SKU wins, otherwise a single visible match. */
function onSearchEnter() {
  const code = search.value.trim();
  if (!code) return;
  const exact = products.value.find((p) => p.barcode === code || p.sku.toLowerCase() === code.toLowerCase());
  const target = exact ?? (visible.value.length === 1 ? visible.value[0] : undefined);
  if (target) {
    addProduct(target);
    search.value = '';
  } else if (!visible.value.length) toast.warning('لا يوجد منتج بهذا الرمز', code);
}

function changeQty(productId: string, qty: number) {
  if (!cart.setQty(productId, qty)) toast.warning('وصلت للحد المتوفر في المخزون');
}

function onDiscountInput(e: Event) {
  const v = Number((e.target as HTMLInputElement).value);
  if (!Number.isFinite(v) || v < 0) cart.discountRate = 0;
  else if (v > maxDiscount.value) {
    cart.discountRate = maxDiscount.value;
    (e.target as HTMLInputElement).value = String(maxDiscount.value);
    toast.warning(`الحد الأقصى للخصم المسموح لك ${maxDiscount.value}%`);
  } else cart.discountRate = v;
}

const customerOptions = computed(() =>
  customers.value.map((c) => ({ value: c.id, label: c.name, sublabel: [c.phone, c.balance > 0 ? `مستحق ${c.balance.toFixed(2)}` : ''].filter(Boolean).join(' · '), keywords: `${c.phone ?? ''} ${c.vatNumber ?? ''}` })),
);
const customer = computed(() => customers.value.find((c) => c.id === cart.customerId));

const draft = computed(() => ({
  customerId: cart.customerId,
  discountRate: cart.discountRate,
  note: cart.note || undefined,
  lines: cart.lines.map((l) => ({ productId: l.productId, qty: l.qty, price: l.price })),
}));

function openCheckout() {
  if (cart.isEmpty) {
    toast.info('السلة فارغة', 'أضف منتجاً أولاً');
    return;
  }
  checkoutOpen.value = true;
}

async function checkout(payment: { paymentMethod: SalePaymentMethod; paidAmount: number; tenderedAmount?: number }) {
  submitting.value = true;
  try {
    const invoice = await createSale({ ...draft.value, ...payment });
    checkoutOpen.value = false;
    completed.value = { invoice, change: payment.tenderedAmount ? changeDue(invoice.grandTotal, payment.tenderedAmount) : 0 };
    cart.clear();
    products.value = await getProducts(); // stock changed
    customers.value = await getCustomers();
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

function printReceipt() {
  if (!completed.value) return;
  router.push({ path: `/print/invoices/${completed.value.invoice.id}`, query: { auto: '1', back: '/pos' } });
}

const anyModal = computed(() => checkoutOpen.value || !!completed.value || shortcutsOpen.value);

/** Run `fn` only when no dialog is open; returning false lets the key keep its default behavior. */
const whenIdle = (fn: () => unknown) => () => {
  if (anyModal.value) return false;
  fn();
};
function bumpActive(delta: number) {
  if (anyModal.value || !cart.activeId) return false;
  changeQty(cart.activeId, cart.qtyInCart(cart.activeId) + delta);
}

useHotkeys({
  F1: () => void (shortcutsOpen.value = !shortcutsOpen.value),
  F2: whenIdle(() => searchInput.value?.focus()),
  F4: whenIdle(() => customerPicker.value?.open()),
  F8: whenIdle(() => discountInput.value?.focus()),
  F9: () => void newSale(),
  F12: whenIdle(openCheckout),
  'ctrl+Enter': whenIdle(openCheckout),
  '+': () => bumpActive(1),
  '-': () => bumpActive(-1),
  // After a completed sale: P prints the receipt, Enter starts the next sale.
  p: () => (completed.value ? printReceipt() : false),
  Enter: () => (completed.value ? void newSale() : false),
});

const shortcuts = [
  ['F2', 'البحث / الباركود'],
  ['F4', 'اختيار العميل'],
  ['F8', 'الخصم'],
  ['F12', 'الدفع'],
  ['F9', 'بيع جديد'],
  ['+ / −', 'كمية الصنف المحدد'],
  ['Enter', 'تأكيد الدفع'],
  ['P', 'طباعة الإيصال بعد البيع'],
  ['F1', 'عرض الاختصارات'],
];
</script>

<template>
  <div class="flex h-screen flex-col overflow-hidden bg-background">
    <!-- Top bar -->
    <header class="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-border px-4">
      <div class="flex items-center gap-3">
        <AppButton size="sm" variant="ghost" :icon="ArrowRight" to="/">لوحة التحكم</AppButton>
        <div class="h-5 w-px bg-border" />
        <span class="text-[13px] font-semibold">نقطة البيع</span>
        <span class="text-[13px] text-text-secondary">{{ settings.settings?.storeName }}</span>
      </div>
      <div class="flex items-center gap-3 text-[13px] text-text-secondary">
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
              class="control h-11 ps-11 text-[15px]"
              placeholder="امسح الباركود أو ابحث باسم المنتج…"
              @keydown.enter.prevent="onSearchEnter"
              @keydown.esc="search = ''"
            />
            <kbd class="num pointer-events-none absolute end-3 top-1/2 -translate-y-1/2 rounded border border-border px-1.5 text-[11px] text-text-secondary">F2</kbd>
          </div>
          <div class="flex gap-1.5 overflow-x-auto pb-0.5">
            <button
              v-for="c in categories"
              :key="c.id"
              type="button"
              class="h-8 shrink-0 rounded-full border px-3.5 text-[13px] transition-colors"
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
              @click="addProduct(p)"
            >
              <span class="line-clamp-2 text-[13px] font-medium leading-snug">{{ p.name }}</span>
              <span class="flex items-end justify-between gap-2">
                <span class="text-[15px] font-semibold"><MoneyText :value="priceOf(p)" /></span>
                <span
                  class="num text-[11px]"
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
      <aside class="flex w-[400px] shrink-0 flex-col border-s border-border bg-surface">
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
            <kbd class="num mb-2 rounded border border-border px-1.5 text-[11px] text-text-secondary">F4</kbd>
          </div>
          <p v-if="customer && customer.balance > 0" class="mt-1.5 text-xs text-warning">رصيد مستحق سابق: <MoneyText :value="customer.balance" /></p>
        </div>

        <div class="min-h-0 flex-1 overflow-y-auto">
          <EmptyState v-if="cart.isEmpty" :icon="ShoppingCart" title="السلة فارغة" description="اضغط على منتج أو امسح الباركود لإضافته" />
          <ul v-else class="divide-y divide-border">
            <li
              v-for="l in cart.lines"
              :key="l.productId"
              class="px-3 py-2.5 transition-colors"
              :class="cart.activeId === l.productId && 'bg-background'"
              @click="cart.activeId = l.productId"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <p class="truncate text-[13px] font-medium">{{ l.name }}</p>
                  <p class="text-xs text-text-secondary"><MoneyText :value="l.price" plain /> × <span class="num">{{ formatNumber(l.qty) }}</span></p>
                </div>
                <div class="flex shrink-0 items-center gap-1">
                  <span class="text-[13px] font-medium"><MoneyText :value="l.price * l.qty" plain /></span>
                  <button type="button" class="rounded p-1 text-text-secondary hover:bg-danger/10 hover:text-danger" aria-label="حذف" @click.stop="cart.remove(l.productId)">
                    <X class="size-3.5" />
                  </button>
                </div>
              </div>
              <div class="mt-1.5 flex items-center gap-1">
                <button type="button" class="flex size-7 items-center justify-center rounded-md border border-border hover:bg-surface-hover" aria-label="إنقاص" @click.stop="changeQty(l.productId, l.qty - 1)">
                  <Minus class="size-3.5" />
                </button>
                <input
                  :value="l.qty"
                  type="number"
                  min="1"
                  class="control h-7 w-14 px-1 text-center"
                  aria-label="الكمية"
                  @change="changeQty(l.productId, Number(($event.target as HTMLInputElement).value))"
                  @click.stop
                />
                <button type="button" class="flex size-7 items-center justify-center rounded-md border border-border hover:bg-surface-hover" aria-label="زيادة" @click.stop="changeQty(l.productId, l.qty + 1)">
                  <Plus class="size-3.5" />
                </button>
              </div>
            </li>
          </ul>
        </div>

        <div class="border-t border-border p-3">
          <div class="mb-2.5 flex items-center justify-between gap-2 text-[13px]">
            <label for="pos-discount" class="text-text-secondary">خصم % <span class="text-[11px]">(حد أقصى <span class="num">{{ maxDiscount }}</span>)</span></label>
            <input
              id="pos-discount"
              ref="discountInput"
              :value="cart.discountRate || ''"
              type="number"
              min="0"
              :max="maxDiscount"
              :disabled="maxDiscount === 0 || cart.isEmpty"
              placeholder="0"
              class="control h-8 w-20"
              @input="onDiscountInput"
            />
          </div>
          <dl class="space-y-1 text-[13px]">
            <div class="flex justify-between"><dt class="text-text-secondary">المجموع (<span class="num">{{ formatNumber(cart.itemCount) }}</span> قطعة)</dt><dd><MoneyText :value="cart.totals.subTotal" plain /></dd></div>
            <div v-if="cart.totals.discountAmount > 0" class="flex justify-between"><dt class="text-text-secondary">الخصم</dt><dd class="text-danger">−<MoneyText :value="cart.totals.discountAmount" plain /></dd></div>
            <div class="flex justify-between"><dt class="text-text-secondary">ضريبة القيمة المضافة <span class="num">{{ settings.salesTaxRate }}%</span></dt><dd><MoneyText :value="cart.totals.taxAmount" plain /></dd></div>
            <div class="flex items-baseline justify-between border-t border-border pt-2">
              <dt class="font-medium">الإجمالي</dt>
              <dd class="text-2xl font-semibold tracking-tight"><MoneyText :value="cart.totals.grandTotal" /></dd>
            </div>
          </dl>
          <div class="mt-3 flex gap-2">
            <AppButton size="lg" :icon="Trash" :disabled="cart.isEmpty" kbd="F9" @click="newSale">إلغاء</AppButton>
            <AppButton size="lg" variant="primary" class="flex-1" :disabled="cart.isEmpty" kbd="F12" @click="openCheckout">الدفع</AppButton>
          </div>
        </div>
      </aside>
    </div>

    <CheckoutModal
      v-model:open="checkoutOpen"
      :total="cart.totals.grandTotal"
      :customer-name="customer?.name"
      :draft="draft"
      :submitting="submitting"
      @confirm="checkout"
    />

    <AppModal :open="!!completed" size="sm" @update:open="(v) => !v && newSale()">
      <div v-if="completed" class="py-2 text-center">
        <CircleCheck class="mx-auto size-12 text-success" :stroke-width="1.5" />
        <p class="mt-3 text-lg font-semibold">تم البيع بنجاح</p>
        <p class="num mt-0.5 text-[13px] text-text-secondary">{{ completed.invoice.number }}</p>
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
        <AppButton :icon="Printer" kbd="P" @click="printReceipt">طباعة الإيصال</AppButton>
        <AppButton variant="primary" kbd="Enter" @click="newSale">بيع جديد</AppButton>
      </template>
    </AppModal>

    <AppModal v-model:open="shortcutsOpen" title="اختصارات لوحة المفاتيح" size="sm">
      <ul class="divide-y divide-border text-[13px]">
        <li v-for="[key, label] in shortcuts" :key="key" class="flex items-center justify-between py-2">
          <span>{{ label }}</span>
          <kbd class="num rounded border border-border bg-surface px-2 py-0.5 text-xs">{{ key }}</kbd>
        </li>
      </ul>
    </AppModal>
  </div>
</template>
