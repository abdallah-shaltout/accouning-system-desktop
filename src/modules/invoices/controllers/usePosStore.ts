import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import type { Product, ProductUnit } from '@/modules/products/types';
import { computeInvoiceTotals } from '../helpers/totals';
import type { HeldSale, SaleInput } from '../types';

/**
 * v2 POS cart line (docs/v2/06-sales-and-pos.md §1). One line per (product, unit, batch, price)
 * combination — scanning the same product/unit again bumps `qty` on the matching line instead of
 * adding a new one; a different unit or a custom price starts a new line (so "سعر معدّل" only tags
 * the line it applies to).
 */
export interface CartLine {
  id: string;
  productId: string;
  name: string;
  sku: string;
  type: Product['type'];
  /** Selected unit (Phase 6's `ProductUnit`), when the product has more than one. Undefined = base unit. */
  unit?: ProductUnit;
  price: number;
  /** Catalog/list price before any override — kept for audit + the "سعر معدّل" tag. */
  listPrice: number;
  priceOverrideReason?: string;
  qty: number;
  discount: number;
  discountIsPct: boolean;
  taxId?: string;
  batchId?: string;
  batchNo?: string;
  /** Stock at the time the product list was loaded, in the BASE unit — used to cap quantities. */
  available: number;
}

/** The POS cart. Lives in Pinia so it survives leaving the POS screen mid-sale. */
export const usePosStore = defineStore('pos', () => {
  const lines = ref<CartLine[]>([]);
  const customerId = ref<string | undefined>();
  /** Invoice-level discount (Shift+F8): either a %, or a flat amount when `discountIsAmount`. */
  const discountRate = ref(0);
  const discountAmount = ref(0);
  const discountIsAmount = ref(false);
  const note = ref('');
  /** Id of the most recently touched line (highlighted, target of +/− shortcuts, F8 line discount). */
  const activeId = ref<string | null>(null);
  /** `n*` qty-multiplier prefix, set by the barcode/search input before the next scan. */
  const qtyPrefix = ref<number | null>(null);
  /** Terminal id for held sales / shifts — one browser tab = one terminal for this mock. */
  const terminalId = ref('pos-1');
  const shiftId = ref<string | undefined>();

  const settings = useSettingsStore();

  const totals = computed(() =>
    computeInvoiceTotals(
      lines.value.map((l) => ({
        qty: l.qty,
        unitPrice: l.price,
        listPrice: l.listPrice,
        discount: l.discount,
        discountIsPct: l.discountIsPct,
        tax: (() => {
          const tax = settings.taxes.find((t) => t.id === (l.taxId ?? settings.settings?.defaultTaxId) && t.active);
          return tax ? { rate: tax.rate, category: tax.category } : { rate: settings.salesTaxRate, category: 'S' as const };
        })(),
      })),
      discountIsAmount.value ? (discountAmount.value > 0 ? { amount: discountAmount.value } : undefined) : discountRate.value > 0 ? { pct: discountRate.value } : undefined,
      settings.settings?.pricesIncludeTax !== false,
    ),
  );
  const itemCount = computed(() => lines.value.reduce((a, l) => a + l.qty, 0));
  const isEmpty = computed(() => lines.value.length === 0);
  const activeLine = computed(() => lines.value.find((l) => l.id === activeId.value) ?? null);

  function lineKey(productId: string, unitId: string | undefined, price: number, batchId: string | undefined) {
    return `${productId}::${unitId ?? 'base'}::${price}::${batchId ?? ''}`;
  }

  function qtyInCart(productId: string) {
    return lines.value.filter((l) => l.productId === productId).reduce((a, l) => a + l.qty * (l.unit?.factor ?? 1), 0);
  }

  /** Consumes and clears the pending `n*` prefix, defaulting to 1. */
  function takeQtyPrefix(): number {
    const q = qtyPrefix.value ?? 1;
    qtyPrefix.value = null;
    return q;
  }

  /** Returns false when stock would be exceeded (in the product's base unit). */
  function add(
    product: Product,
    opts: { price: number; listPrice?: number; qty?: number; unit?: ProductUnit; priceOverrideReason?: string; batchId?: string; batchNo?: string; taxId?: string } = { price: product.price },
  ): boolean {
    const qty = opts.qty ?? 1;
    const key = lineKey(product.id, opts.unit?.id, opts.price, opts.batchId);
    const existing = lines.value.find((l) => lineKey(l.productId, l.unit?.id, l.price, l.batchId) === key);
    const factor = opts.unit?.factor ?? 1;
    const nextBaseQty = qtyInCart(product.id) + qty * factor;
    if (product.type === 'product' && nextBaseQty > product.stockQty) return false;
    if (existing) {
      existing.qty += qty;
    } else {
      lines.value.push({
        id: `cl-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
        productId: product.id,
        name: product.name,
        sku: product.sku,
        type: product.type,
        unit: opts.unit,
        price: opts.price,
        listPrice: opts.listPrice ?? product.price,
        priceOverrideReason: opts.priceOverrideReason,
        qty,
        discount: 0,
        discountIsPct: true,
        taxId: opts.taxId ?? product.saleTaxId,
        batchId: opts.batchId,
        batchNo: opts.batchNo,
        available: product.stockQty,
      });
      activeId.value = lines.value.at(-1)!.id;
      return true;
    }
    activeId.value = existing.id;
    return true;
  }

  function setQty(lineId: string, qty: number): boolean {
    const line = lines.value.find((l) => l.id === lineId);
    if (!line) return false;
    if (qty <= 0) {
      remove(lineId);
      return true;
    }
    const factor = line.unit?.factor ?? 1;
    const otherBaseQty = qtyInCart(line.productId) - line.qty * factor;
    if (line.type === 'product' && (qty * factor + otherBaseQty) > line.available) {
      line.qty = Math.max(1, Math.floor((line.available - otherBaseQty) / factor));
      return false;
    }
    line.qty = qty;
    activeId.value = lineId;
    return true;
  }

  function setLineDiscount(lineId: string, discount: number, isPct: boolean) {
    const line = lines.value.find((l) => l.id === lineId);
    if (line) {
      line.discount = discount;
      line.discountIsPct = isPct;
    }
  }

  function remove(lineId: string) {
    lines.value = lines.value.filter((l) => l.id !== lineId);
    if (activeId.value === lineId) activeId.value = lines.value.at(-1)?.id ?? null;
  }

  function clear() {
    lines.value = [];
    customerId.value = undefined;
    discountRate.value = 0;
    discountAmount.value = 0;
    discountIsAmount.value = false;
    note.value = '';
    activeId.value = null;
    qtyPrefix.value = null;
  }

  /** Builds the `SaleInput` shape sent to `createSale`, independent of payment fields. */
  function toSaleLines(): SaleInput['lines'] {
    return lines.value.map((l) => ({
      productId: l.productId,
      qty: l.qty,
      price: l.price,
      discount: l.discount || undefined,
      discountIsPct: l.discountIsPct,
      taxId: l.taxId,
      unitId: l.unit?.id,
      unitFactor: l.unit?.factor,
      listPrice: l.price !== l.listPrice ? l.listPrice : undefined,
      priceOverrideReason: l.priceOverrideReason,
      batchId: l.batchId,
      batchNo: l.batchNo,
    }));
  }

  function toHeld(label: string | undefined): Omit<HeldSale, 'id' | 'heldAt' | 'heldBy'> {
    return {
      label,
      terminalId: terminalId.value,
      customerId: customerId.value,
      discountRate: discountIsAmount.value ? 0 : discountRate.value,
      discountIsPct: !discountIsAmount.value,
      note: note.value || undefined,
      lines: lines.value.map((l) => ({
        productId: l.productId,
        unitId: l.unit?.id,
        qty: l.qty,
        price: l.price,
        listPrice: l.listPrice !== l.price ? l.listPrice : undefined,
        priceOverrideReason: l.priceOverrideReason,
        discount: l.discount || undefined,
        discountIsPct: l.discountIsPct,
        batchId: l.batchId,
        taxId: l.taxId,
      })),
    };
  }

  /** Restores a held sale into the cart — needs the live product list to re-resolve unit/stock info. */
  function restoreHeld(held: HeldSale, products: Product[]) {
    clear();
    customerId.value = held.customerId;
    discountRate.value = held.discountIsPct ? held.discountRate : 0;
    discountAmount.value = held.discountIsPct ? 0 : held.discountRate;
    discountIsAmount.value = !held.discountIsPct;
    note.value = held.note ?? '';
    for (const hl of held.lines) {
      const product = products.find((p) => p.id === hl.productId);
      if (!product) continue;
      const unit = hl.unitId ? product.units?.find((u) => u.id === hl.unitId) : undefined;
      add(product, {
        price: hl.price,
        listPrice: hl.listPrice,
        qty: hl.qty,
        unit,
        priceOverrideReason: hl.priceOverrideReason,
        batchId: hl.batchId,
        taxId: hl.taxId,
      });
      const line = lines.value.at(-1);
      if (line && hl.discount) {
        line.discount = hl.discount;
        line.discountIsPct = hl.discountIsPct ?? true;
      }
    }
  }

  return {
    lines,
    customerId,
    discountRate,
    discountAmount,
    discountIsAmount,
    note,
    activeId,
    activeLine,
    qtyPrefix,
    terminalId,
    shiftId,
    totals,
    itemCount,
    isEmpty,
    qtyInCart,
    takeQtyPrefix,
    add,
    setQty,
    setLineDiscount,
    remove,
    clear,
    toSaleLines,
    toHeld,
    restoreHeld,
  };
});
