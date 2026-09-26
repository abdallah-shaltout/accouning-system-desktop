<script setup lang="ts" generic="L extends Record<string, any>">
/**
 * v2 doc 17 Phase F-0 (F1 "LineItemsEditor" — replaces 6 copies of the line grid: invoice, purchase,
 * journal, stock adjustment, stock count, transfer). Generic over a line type `L` with a declared
 * column config; keyboard entry (Enter = next cell, Ctrl+Enter = new line, Del on an empty/focused
 * row = remove it); emits line changes and a totals request to the parent.
 *
 * IMPORTANT — this component only renders rows and emits events. It does **not** compute totals,
 * VAT or discounts itself: every number a cell shows comes from the row data the parent already
 * computed with the existing helpers (e.g. `modules/invoices/helpers/totals.ts`, which stays the
 * single source of tax-inclusive VAT + line→invoice→VAT discount order —
 * `docs/v2/02-accounting-review.md`). `@lines-change` just reports the raw row edits back; the
 * parent re-runs its own totals helper and passes the recomputed rows back in via `lines`.
 *
 * Not wired into any real form yet (invoice/purchase/journal/adjustment/count/transfer forms are
 * F-2, out of scope for F-0) — this is the building block those pages will adopt later.
 */
import { computed, nextTick, ref } from 'vue';
import { Plus, Trash2 } from '@lucide/vue';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/modules/core/components/shadcn/table';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';

export interface LineColumn<Line> {
  key: keyof Line & string;
  label: string;
  /** Cell input type — `custom` lets the parent render its own control via the `cell-<key>` slot
   *  (e.g. a product/account picker) while this component still owns keyboard nav for that cell. */
  type?: 'text' | 'number' | 'custom';
  align?: 'start' | 'center' | 'end';
  width?: string;
  readonly?: boolean;
  placeholder?: string;
}

const props = defineProps<{
  lines: L[];
  columns: LineColumn<L>[];
  /** Factory for a brand-new empty row (no math — just the shape), used by "add line" and Ctrl+Enter. */
  newLine: () => L;
  rowKey?: keyof L & string;
  disabled?: boolean;
  emptyLabel?: string;
}>();

const emit = defineEmits<{
  /** The full, still-unaggregated line array after any add/edit/remove — parent recomputes totals from this. */
  'lines-change': [lines: L[]];
}>();

const rowKeyName = computed(() => props.rowKey ?? ('id' as keyof L & string));
const tableRef = ref<HTMLElement>();

function updateCell(rowIndex: number, key: keyof L & string, value: unknown) {
  const next = props.lines.map((l, i) => (i === rowIndex ? { ...l, [key]: value } : l));
  emit('lines-change', next);
}

function addLine() {
  emit('lines-change', [...props.lines, props.newLine()]);
  void nextTick(() => focusCell(props.lines.length, 0));
}

function removeLine(rowIndex: number) {
  emit('lines-change', props.lines.filter((_, i) => i !== rowIndex));
}

function focusCell(rowIndex: number, colIndex: number) {
  const el = tableRef.value?.querySelector<HTMLElement>(`[data-cell="${rowIndex}-${colIndex}"] input, [data-cell="${rowIndex}-${colIndex}"] [tabindex]`);
  el?.focus();
}

function onKeydown(e: KeyboardEvent, rowIndex: number, colIndex: number) {
  if (props.disabled) return;
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    addLine();
    return;
  }
  if (e.key === 'Enter') {
    e.preventDefault();
    const isLastCol = colIndex >= props.columns.length - 1;
    if (isLastCol && rowIndex >= props.lines.length - 1) addLine();
    else if (isLastCol) void nextTick(() => focusCell(rowIndex + 1, 0));
    else void nextTick(() => focusCell(rowIndex, colIndex + 1));
    return;
  }
  if (e.key === 'Delete' && (e.ctrlKey || e.metaKey) && props.lines.length > 1) {
    e.preventDefault();
    removeLine(rowIndex);
  }
}
</script>

<template>
  <div class="overflow-hidden rounded-xl border border-border">
    <Table ref="tableRef" class="w-full border-collapse text-body" container-class="overflow-x-auto">
      <TableHeader>
        <TableRow class="bg-surface hover:bg-surface">
          <TableHead
            v-for="col in columns"
            :key="col.key"
            scope="col"
            class="h-auto whitespace-nowrap border-b border-border px-3 py-2.5 text-xs font-medium text-text-secondary"
            :class="col.align === 'end' ? 'text-end' : col.align === 'center' ? 'text-center' : 'text-start'"
            :style="col.width ? { width: col.width } : undefined"
          >
            {{ col.label }}
          </TableHead>
          <TableHead scope="col" class="no-print w-10 border-b border-border" />
        </TableRow>
      </TableHeader>
      <TableBody v-if="!lines.length">
        <TableRow>
          <TableCell :colspan="columns.length + 1" class="whitespace-normal">
            <EmptyState compact :title="emptyLabel ?? 'لا توجد بنود بعد'" />
          </TableCell>
        </TableRow>
      </TableBody>
      <TableBody v-else>
        <TableRow v-for="(line, rowIndex) in lines" :key="line[rowKeyName] ?? rowIndex" class="border-b border-border last:border-0">
          <TableCell
            v-for="(col, colIndex) in columns"
            :key="col.key"
            :data-cell="`${rowIndex}-${colIndex}`"
            class="px-2 py-1.5 align-middle"
            :class="col.align === 'end' ? 'text-end' : col.align === 'center' ? 'text-center' : 'text-start'"
            @keydown="onKeydown($event, rowIndex, colIndex)"
          >
            <slot :name="`cell-${col.key}`" :line="line" :row-index="rowIndex" :update="(v: unknown) => updateCell(rowIndex, col.key, v)">
              <AppInput
                v-if="col.type === 'number'"
                :model-value="line[col.key]"
                type="number"
                :disabled="disabled || col.readonly"
                :placeholder="col.placeholder"
                input-class="text-end"
                @update:model-value="(v) => updateCell(rowIndex, col.key, v)"
              />
              <AppInput
                v-else
                :model-value="line[col.key]"
                :disabled="disabled || col.readonly"
                :placeholder="col.placeholder"
                @update:model-value="(v) => updateCell(rowIndex, col.key, v)"
              />
            </slot>
          </TableCell>
          <TableCell class="no-print px-2 py-1.5 text-center">
            <button
              type="button"
              class="rounded p-1 text-text-secondary hover:bg-danger/10 hover:text-danger disabled:opacity-40"
              :disabled="disabled || lines.length <= 1"
              aria-label="حذف السطر"
              @click="removeLine(rowIndex)"
            >
              <Trash2 class="size-4" />
            </button>
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
    <div class="no-print border-t border-border bg-surface px-3 py-2">
      <AppButton size="sm" variant="ghost" :icon="Plus" :disabled="disabled" @click="addLine">إضافة سطر</AppButton>
    </div>
  </div>
</template>
