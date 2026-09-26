<script setup lang="ts">
/**
 * v2 doc 17 Phase F-2: page-specific line grid for InvoiceFormPage, extracted to keep the page under
 * the ~250-line guideline (CLAUDE.md UI rule 12). Render/emit-only, same as `LineItemsEditor` itself
 * — `totals` (computeInvoiceTotals) is computed by the page; this component never adds/subtracts.
 *
 * Kept as a custom overlay ON TOP of `LineItemsEditor` rather than forced fully into its generic
 * cell model, because this grid has three behaviors `LineItemsEditor` doesn't (and shouldn't) know
 * about: a `<datalist>`-based product resolution via the native `change` event (no click handler on
 * datalist options — see `resolveLineByName` on the page), a per-line free-text toggle, and
 * paste-from-Excel across MULTIPLE rows (a single paste can append new rows past the one it started
 * in). `LineItemsEditor`'s own Enter/Ctrl+Enter keyboard model still comes for free since every
 * column here renders through its `cell-*` slots and its `newLine`/`lines-change` contract.
 */
import type { Product } from '@/modules/products/types';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import LineItemsEditor, { type LineColumn } from '@/modules/core/components/blocks/LineItemsEditor.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';

export interface DeskLine {
  id: string;
  productId?: string;
  name: string;
  unitId?: string;
  qty: number;
  price: number;
  discount: number;
  discountIsPct: boolean;
  taxId?: string;
  isFreeText: boolean;
  revenueAccountId?: string;
  net?: never;
  vat?: never;
  gross?: never;
}

const props = defineProps<{
  lines: DeskLine[];
  columns: LineColumn<DeskLine>[];
  newLine: () => DeskLine;
  products: Product[];
  revenueAccounts: { id: string; code: string; name: string }[];
  lineTotals: { net: number; vat: number; gross: number }[];
}>();

const emit = defineEmits<{
  'lines-change': [lines: DeskLine[]];
  'add-line': [];
  'duplicate-line': [index: number];
  'remove-line': [index: number];
}>();

function pickProduct(line: DeskLine, product: Product) {
  line.productId = product.id;
  line.name = product.name;
  line.price = product.price;
  line.taxId = product.saleTaxId;
  line.isFreeText = false;
}

/** `<datalist>` options can't carry a click handler (native browser UI) — the only observable signal
 *  when a suggestion is chosen is the `<input>`'s value changing to match an option exactly. Resolve
 *  the product on every `name` change instead of relying on a click from the option itself: an exact
 *  name match auto-attaches the product id/price/tax; anything else is left free-text. */
function resolveLineByName(line: DeskLine) {
  const typed = line.name.trim();
  if (!typed) return;
  const match = props.products.find((p) => p.name === typed);
  if (match) pickProduct(line, match);
}

function toggleFreeText(line: DeskLine) {
  line.isFreeText = !line.isFreeText;
  if (line.isFreeText) line.productId = undefined;
}

/** Paste from Excel (docs/v2/06 §2): tab/newline-separated rows -> name / qty / price / discount%,
 *  appending new rows past `startIndex` as needed — this is the one behavior a single-cell
 *  `LineItemsEditor` slot can't do on its own, so it mutates `lines` directly and emits the result. */
function onPaste(e: ClipboardEvent, startIndex: number) {
  const text = e.clipboardData?.getData('text/plain');
  if (!text?.includes('\t') && !text?.includes('\n')) return;
  e.preventDefault();
  const rows = text
    .split(/\r?\n/)
    .filter((r) => r.trim())
    .map((r) => r.split('\t'));
  const next = props.lines.map((l) => ({ ...l }));
  rows.forEach((cells, i) => {
    const idx = startIndex + i;
    if (!next[idx]) next.push(props.newLine());
    const l = next[idx];
    l.name = cells[0]?.trim() ?? l.name;
    l.qty = num0(cells[1]) || l.qty || 1;
    l.price = num0(cells[2]) || l.price;
    l.discount = num0(cells[3]) || 0;
    l.isFreeText = true;
  });
  emit('lines-change', next);
}
</script>

<template>
  <LineItemsEditor
    :lines="lines"
    :columns="columns"
    :new-line="newLine"
    row-key="id"
    @lines-change="(next) => emit('lines-change', next)"
  >
    <template #cell-name="{ line, rowIndex }">
      <input
        v-model="line.name"
        class="control h-9 w-full"
        :placeholder="line.isFreeText ? 'وصف حر' : 'ابحث عن منتج أو اكتب وصفاً'"
        :list="`prod-list-${rowIndex}`"
        @paste="(e) => onPaste(e, rowIndex)"
        @change="resolveLineByName(line)"
        @keydown.ctrl.enter.prevent="emit('add-line')"
        @keydown.ctrl.d.prevent="emit('duplicate-line', rowIndex)"
        @keydown.ctrl.delete.prevent="emit('remove-line', rowIndex)"
      />
      <datalist :id="`prod-list-${rowIndex}`">
        <option v-for="p in products" :key="p.id" :value="p.name" />
      </datalist>
      <button type="button" class="mt-1 text-tiny text-text-secondary hover:text-primary" @click="toggleFreeText(line)">
        {{ line.isFreeText ? 'سطر حر ✓' : 'سطر حر؟' }}
      </button>
      <AppSelect
        v-if="line.isFreeText"
        v-model="line.revenueAccountId"
        class="mt-1"
        :options="revenueAccounts.map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` }))"
        placeholder="حساب الإيراد (مطلوب)"
      />
    </template>
    <template #cell-qty="{ line }">
      <input
        :value="line.qty"
        type="number"
        min="0"
        step="0.001"
        class="control h-9 w-full"
        @input="(e) => (line.qty = Number((e.target as HTMLInputElement).value) || 0)"
      />
    </template>
    <template #cell-price="{ line }">
      <input
        :value="line.price"
        type="number"
        min="0"
        step="0.01"
        class="control h-9 w-full"
        @input="(e) => (line.price = Number((e.target as HTMLInputElement).value) || 0)"
      />
    </template>
    <template #cell-discount="{ line }">
      <input
        :value="line.discount"
        type="number"
        min="0"
        class="control h-9 w-full"
        @input="(e) => (line.discount = Number((e.target as HTMLInputElement).value) || 0)"
      />
    </template>
    <template #cell-net="{ rowIndex }">
      <span class="num text-text-secondary">{{ formatNumber(lineTotals[rowIndex]?.net ?? 0) }}</span>
    </template>
    <template #cell-vat="{ rowIndex }">
      <span class="num text-text-secondary">{{ formatNumber(lineTotals[rowIndex]?.vat ?? 0) }}</span>
    </template>
    <template #cell-gross="{ rowIndex }">
      <span class="num font-medium">{{ formatNumber(lineTotals[rowIndex]?.gross ?? 0) }}</span>
    </template>
  </LineItemsEditor>
</template>
