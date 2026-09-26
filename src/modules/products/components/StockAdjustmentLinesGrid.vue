<script setup lang="ts">
/**
 * v2 doc 17 Phase F-2: page-specific line grid for StockAdjustmentFormPage, extracted to keep the
 * page under the ~250-line guideline (CLAUDE.md UI rule 12). Render/emit-only, same as
 * `LineItemsEditor` itself — every number (`change`, `value`, totals) is computed by the page and
 * passed in; this component never adds/subtracts.
 */
import type { Product } from '../types';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import LineItemsEditor, { type LineColumn } from '@/modules/core/components/blocks/LineItemsEditor.vue';
import { formatNumber } from '@/modules/core/helpers/format';

export interface AdjustmentLine {
  key: number;
  productId?: string;
  qty?: number;
  counted?: number;
  batchNo?: string;
  expiryDate?: string;
  systemQty?: never;
  diff?: never;
  costPrice?: never;
  value?: never;
}

const props = defineProps<{
  lines: AdjustmentLine[];
  columns: LineColumn<AdjustmentLine>[];
  newLine: () => AdjustmentLine;
  type: 'STOCK_IN' | 'LOSS' | 'STOCKTAKE';
  byId: Map<string, Product>;
  productOptions: { value: string; label: string; sublabel?: string; keywords?: string }[];
  lineErrors: Record<number, string>;
  change: (line: AdjustmentLine) => number;
  value: (line: AdjustmentLine) => number;
}>();

const emit = defineEmits<{ 'lines-change': [lines: AdjustmentLine[]] }>();
</script>

<template>
  <LineItemsEditor
    :lines="lines"
    :columns="columns"
    :new-line="newLine"
    row-key="key"
    empty-label="لا توجد أصناف في هذا النطاق"
    :hide-actions="type === 'STOCKTAKE'"
    :hide-add-button="type === 'STOCKTAKE'"
    @lines-change="(next) => emit('lines-change', next)"
  >
    <template #cell-productId="{ line }">
      <template v-if="type !== 'STOCKTAKE'">
        <AppCombobox
          :model-value="line.productId"
          :options="productOptions"
          placeholder="اختر صنفاً…"
          search-placeholder="اسم، SKU، أو باركود"
          dense
          @update:model-value="(v) => (line.productId = v)"
        />
      </template>
      <span v-else>
        {{ byId.get(line.productId!)?.name }}
        <span class="num block text-tiny text-text-secondary">{{ byId.get(line.productId!)?.sku }}</span>
      </span>
    </template>
    <template #cell-systemQty="{ line }">
      <span class="num text-text-secondary">{{ line.productId ? formatNumber(byId.get(line.productId)?.stockQty) : '—' }}</span>
    </template>
    <template #cell-qty="{ line }">
      <AppInput
        v-if="type !== 'STOCKTAKE'"
        :model-value="line.qty"
        type="number"
        min="0"
        :error="lineErrors[line.key]"
        @update:model-value="(v) => (line.qty = v === '' || v === null ? undefined : Number(v))"
      />
    </template>
    <template #cell-counted="{ line }">
      <AppInput
        v-if="type === 'STOCKTAKE'"
        :model-value="line.counted"
        type="number"
        min="0"
        :error="lineErrors[line.key]"
        @update:model-value="(v) => (line.counted = v === '' || v === null ? undefined : Number(v))"
      />
    </template>
    <template #cell-diff="{ line }">
      <span class="num font-medium" :class="change(line) > 0 ? 'text-success' : change(line) < 0 ? 'text-danger' : 'text-text-secondary'">
        {{ change(line) > 0 ? '+' : '' }}{{ formatNumber(change(line)) }}
      </span>
    </template>
    <template #cell-batchNo="{ line }">
      <template v-if="line.productId && byId.get(line.productId)?.trackBatches">
        <AppInput v-model="line.batchNo" class="mb-1 w-32" ltr placeholder="رقم التشغيلة" />
        <AppDatePicker v-model="line.expiryDate" class="w-32" compact />
      </template>
      <span v-else class="text-xs text-text-secondary">—</span>
    </template>
    <template #cell-costPrice="{ line }">
      <MoneyText v-if="line.productId" :value="byId.get(line.productId)?.costPrice" plain class="text-text-secondary" />
    </template>
    <template #cell-value="{ line }">
      <MoneyText :value="value(line)" plain :signed="type === 'STOCKTAKE'" dash-zero />
    </template>
  </LineItemsEditor>
</template>
