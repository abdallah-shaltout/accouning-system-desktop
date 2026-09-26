<script setup lang="ts">
import { computed } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { round2 } from '@/modules/core/helpers/numbers';
import { useCatalogStore } from '../controllers/useCatalogStore';
import type { ProductUnit, ProductUnitPrice } from '../types';

/**
 * v2 §1 "الأسعار" — price-list × unit matrix. Cell = `unitPrices` override for (priceListId, unitId)
 * — empty defaults to `basePrice(unit) = baseListPrice(priceList) × factor`, shown "تلقائي" (auto,
 * blue) until edited. The base price-list column itself is `basePrice` (from `price`/`unitPrice`
 * fields on the parent form) — this component only handles the *extra* price lists.
 */
const props = defineProps<{ basePrice: number; baseCost: number; units: ProductUnit[]; disabled?: boolean }>();
const unitPrices = defineModel<ProductUnitPrice[]>({ default: () => [] });

const catalog = useCatalogStore();

/** The base unit's row (factor 1) always exists implicitly even with no extra units defined. */
const rows = computed<{ unitId: string; label: string; factor: number }[]>(() => {
  if (!props.units.length) return [{ unitId: '__base__', label: 'الوحدة الأساسية', factor: 1 }];
  return props.units
    .filter((u) => u.active)
    .map((u) => ({ unitId: u.id, label: catalog.unitName(u.unitId) || '—', factor: u.factor }));
});

function cell(priceListId: string, unitId: string): number | undefined {
  return unitPrices.value.find((x) => x.priceListId === priceListId && x.unitId === unitId)?.value;
}

function autoPrice(factor: number): number {
  return round2(props.basePrice * factor);
}

function setCell(priceListId: string, unitId: string, value: number | undefined) {
  const others = unitPrices.value.filter((x) => !(x.priceListId === priceListId && x.unitId === unitId));
  unitPrices.value = value === undefined || Number.isNaN(value) ? others : [...others, { priceListId, unitId, value }];
}

function margin(price: number): number | null {
  if (!price || !props.baseCost) return null;
  return ((price - props.baseCost) / price) * 100;
}
</script>

<template>
  <div class="overflow-x-auto rounded-lg border border-border">
    <table class="w-full text-body">
      <thead class="bg-surface text-xs text-text-secondary">
        <tr class="border-b border-border">
          <th class="px-3 py-2 text-start font-medium">قائمة الأسعار</th>
          <th v-for="r in rows" :key="r.unitId" class="px-3 py-2 text-start font-medium">{{ r.label }}</th>
        </tr>
      </thead>
      <tbody>
        <tr class="border-b border-border bg-background/40">
          <td class="px-3 py-2 font-medium">السعر الأساسي</td>
          <td v-for="r in rows" :key="r.unitId" class="px-3 py-2">
            <span v-if="r.factor === 1" class="num"><MoneyText :value="basePrice" plain /></span>
            <span v-else class="num text-text-secondary">
              <MoneyText :value="autoPrice(r.factor)" plain /> <span class="text-tiny text-primary">(تلقائي ×{{ formatNumber(r.factor) }})</span>
            </span>
          </td>
        </tr>
        <tr v-for="pl in catalog.priceLists" :key="pl.id" class="border-b border-border last:border-0">
          <td class="px-3 py-2" :class="!pl.active && 'text-text-secondary'">{{ pl.name }}</td>
          <td v-for="r in rows" :key="r.unitId" class="group relative px-3 py-1.5">
            <input
              :value="cell(pl.id, r.unitId)"
              type="number"
              min="0"
              :disabled="disabled"
              class="control h-8 w-28"
              :placeholder="String(autoPrice(r.factor))"
              :title="margin(cell(pl.id, r.unitId) ?? autoPrice(r.factor)) !== null ? `الهامش ${formatNumber(margin(cell(pl.id, r.unitId) ?? autoPrice(r.factor))!, 1)}%` : undefined"
              @input="setCell(pl.id, r.unitId, ($event.target as HTMLInputElement).value === '' ? undefined : Number(($event.target as HTMLInputElement).value))"
            />
            <span v-if="cell(pl.id, r.unitId) === undefined" class="pointer-events-none absolute end-4 top-1/2 -translate-y-1/2 text-tiny text-primary">تلقائي</span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
