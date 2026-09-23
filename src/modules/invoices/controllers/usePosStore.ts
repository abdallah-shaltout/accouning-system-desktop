import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import type { Product } from '@/modules/products/types';
import { computeSaleTotals } from '../helpers/totals';

export interface CartLine {
  productId: string;
  name: string;
  sku: string;
  price: number;
  qty: number;
  type: Product['type'];
  /** Stock at the time the product list was loaded — used to cap quantities. */
  available: number;
}

/** The POS cart. Lives in Pinia so it survives leaving the POS screen mid-sale. */
export const usePosStore = defineStore('pos', () => {
  const lines = ref<CartLine[]>([]);
  const customerId = ref<string | undefined>();
  const discountRate = ref(0);
  const note = ref('');
  /** Id of the most recently touched line (highlighted, target of +/− shortcuts). */
  const activeId = ref<string | null>(null);

  const settings = useSettingsStore();
  const totals = computed(() => computeSaleTotals(lines.value, discountRate.value, settings.salesTaxRate));
  const itemCount = computed(() => lines.value.reduce((a, l) => a + l.qty, 0));
  const isEmpty = computed(() => lines.value.length === 0);

  function qtyInCart(productId: string) {
    return lines.value.find((l) => l.productId === productId)?.qty ?? 0;
  }

  /** Returns false when stock would be exceeded. */
  function add(product: Product, price: number, qty = 1): boolean {
    const existing = lines.value.find((l) => l.productId === product.id);
    const nextQty = (existing?.qty ?? 0) + qty;
    if (product.type === 'product' && nextQty > product.stockQty) return false;
    if (existing) {
      existing.qty = nextQty;
      existing.available = product.stockQty;
    } else {
      lines.value.push({ productId: product.id, name: product.name, sku: product.sku, price, qty, type: product.type, available: product.stockQty });
    }
    activeId.value = product.id;
    return true;
  }

  function setQty(productId: string, qty: number): boolean {
    const line = lines.value.find((l) => l.productId === productId);
    if (!line) return false;
    if (qty <= 0) {
      remove(productId);
      return true;
    }
    if (line.type === 'product' && qty > line.available) {
      line.qty = line.available;
      return false;
    }
    line.qty = qty;
    activeId.value = productId;
    return true;
  }

  function remove(productId: string) {
    lines.value = lines.value.filter((l) => l.productId !== productId);
    if (activeId.value === productId) activeId.value = lines.value.at(-1)?.productId ?? null;
  }

  function clear() {
    lines.value = [];
    customerId.value = undefined;
    discountRate.value = 0;
    note.value = '';
    activeId.value = null;
  }

  return { lines, customerId, discountRate, note, activeId, totals, itemCount, isEmpty, qtyInCart, add, setQty, remove, clear };
});
