import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as catalog from '../services/catalogService';
import type { Category, PriceList, Unit } from '../types';

/** Categories, units and price lists — small lookup tables shared by many screens. */
export const useCatalogStore = defineStore('catalog', () => {
  const categories = ref<(Category & { productCount: number })[]>([]);
  const units = ref<(Unit & { productCount: number })[]>([]);
  const priceLists = ref<PriceList[]>([]);
  const loaded = ref(false);
  let pending: Promise<void> | null = null;

  const categoryName = computed(() => {
    const map = new Map(categories.value.map((c) => [c.id, c.name]));
    return (id?: string) => (id ? map.get(id) ?? '—' : '—');
  });
  const unitName = computed(() => {
    const map = new Map(units.value.map((u) => [u.id, u.name]));
    return (id?: string) => (id ? map.get(id) ?? '' : '');
  });

  async function load(force = false) {
    if (loaded.value && !force) return;
    if (pending && !force) return pending;
    pending = (async () => {
      const [c, u, p] = await Promise.all([catalog.getCategories(), catalog.getUnits(), catalog.getPriceLists()]);
      categories.value = c;
      units.value = u;
      priceLists.value = p;
      loaded.value = true;
    })();
    try {
      await pending;
    } finally {
      pending = null;
    }
  }

  return { categories, units, priceLists, loaded, categoryName, unitName, load };
});
