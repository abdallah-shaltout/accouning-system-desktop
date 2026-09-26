<script setup lang="ts">
/**
 * v2 doc 17 Phase F-2: page-specific line grid for PurchaseFormPage, extracted to keep the page
 * under the ~250-line guideline (CLAUDE.md UI rule 12). Render/emit-only, same as `LineItemsEditor`
 * itself — `lineTotal` is the page's existing helper; this component never adds/subtracts.
 */
import type { Product } from '@/modules/products/types';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import LineItemsEditor, { type LineColumn } from '@/modules/core/components/blocks/LineItemsEditor.vue';

export interface PurchaseLine {
  key: number;
  productId?: string;
  unitId?: string;
  qty?: number;
  costPrice?: number;
  discount?: number;
  discountIsPct?: boolean;
  taxId?: string;
  unit?: never;
  total?: never;
}

const props = defineProps<{
  lines: PurchaseLine[];
  columns: LineColumn<PurchaseLine>[];
  newLine: () => PurchaseLine;
  byId: Map<string, Product>;
  productOptions: { value: string; label: string; sublabel?: string; keywords?: string }[];
  taxOptions: { value: string; label: string }[];
  unitOptionsFor: (productId: string | undefined) => { value: string; label: string }[];
  lineTotal: (l: PurchaseLine) => number;
}>();

const emit = defineEmits<{
  'lines-change': [lines: PurchaseLine[]];
  'product-selected': [line: PurchaseLine];
}>();
</script>

<template>
  <LineItemsEditor
    :lines="lines"
    :columns="columns"
    :new-line="newLine"
    row-key="key"
    @lines-change="(next) => emit('lines-change', next)"
  >
    <template #cell-productId="{ line }">
      <AppCombobox
        :model-value="line.productId"
        :options="productOptions"
        placeholder="اختر صنفاً…"
        search-placeholder="اسم أو SKU أو باركود"
        dense
        @update:model-value="(v) => { line.productId = v; emit('product-selected', line); }"
      />
    </template>
    <template #cell-unit="{ line }">
      <select v-if="unitOptionsFor(line.productId).length" v-model="line.unitId" class="control h-8 text-xs">
        <option v-for="o in unitOptionsFor(line.productId)" :key="o.value" :value="o.value">{{ o.label }}</option>
      </select>
      <span v-else class="text-tiny text-text-secondary">أساسية</span>
    </template>
    <template #cell-qty="{ line }">
      <input
        :value="line.qty"
        type="number"
        min="0.01"
        step="any"
        class="control num h-8"
        @input="(e) => (line.qty = Number((e.target as HTMLInputElement).value) || undefined)"
      />
    </template>
    <template #cell-costPrice="{ line }">
      <input
        :value="line.costPrice"
        type="number"
        min="0"
        step="0.01"
        class="control num h-8"
        @input="(e) => (line.costPrice = Number((e.target as HTMLInputElement).value) || undefined)"
      />
    </template>
    <template #cell-discount="{ line }">
      <div class="flex gap-1">
        <input
          :value="line.discount"
          type="number"
          min="0"
          step="0.01"
          class="control num h-8 w-14"
          placeholder="0"
          @input="(e) => (line.discount = Number((e.target as HTMLInputElement).value) || undefined)"
        />
        <button type="button" class="rounded border border-border px-1.5 text-caption text-text-secondary hover:bg-surface-hover" @click="line.discountIsPct = !line.discountIsPct">
          {{ line.discountIsPct ? '%' : 'ر.س' }}
        </button>
      </div>
    </template>
    <template #cell-taxId="{ line }">
      <select v-model="line.taxId" class="control h-8 text-xs">
        <option v-for="o in taxOptions" :key="o.value" :value="o.value">{{ o.label }}</option>
      </select>
    </template>
    <template #cell-total="{ line }">
      <MoneyText :value="lineTotal(line)" plain dash-zero />
    </template>
  </LineItemsEditor>
</template>
