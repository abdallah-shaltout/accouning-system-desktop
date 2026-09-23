<script setup lang="ts" generic="T extends Record<string, any>">
import { computed, ref, watch, type Component } from 'vue';
import { ArrowDown, ArrowUp, ChevronLeft, ChevronRight } from '@lucide/vue';
import { formatNumber } from '../../helpers/format';
import EmptyState from './EmptyState.vue';
import ErrorState from './ErrorState.vue';

export interface Column<R = any> {
  key: string;
  label: string;
  /** Numeric columns: LTR tabular digits, aligned to the start (right) edge so decimals line up. */
  numeric?: boolean;
  align?: 'start' | 'center' | 'end';
  width?: string;
  sortable?: boolean;
  sortValue?: (row: R) => string | number;
  class?: string;
  /** Hidden when printing / exporting? (e.g. action columns) */
  noPrint?: boolean;
}

const props = withDefaults(
  defineProps<{
    columns: Column<T>[];
    rows: T[] | undefined;
    rowKey?: string;
    loading?: boolean;
    error?: string | null;
    clickable?: boolean;
    /** 0 disables paging. */
    pageSize?: number;
    emptyTitle?: string;
    emptyDescription?: string;
    emptyIcon?: Component;
    /** Highlight a row (e.g. just-created record). */
    highlightKey?: string;
    skeletonRows?: number;
    sticky?: boolean;
  }>(),
  { rowKey: 'id', pageSize: 25, skeletonRows: 8, emptyTitle: 'لا توجد سجلات' },
);

const emit = defineEmits<{ 'row-click': [row: T]; retry: [] }>();

const sortKey = ref<string | null>(null);
const sortDir = ref<'asc' | 'desc'>('asc');
const page = ref(1);

const sorted = computed(() => {
  const rows = props.rows ?? [];
  if (!sortKey.value) return rows;
  const col = props.columns.find((c) => c.key === sortKey.value);
  const get = col?.sortValue ?? ((r: T) => r[sortKey.value!]);
  const dir = sortDir.value === 'asc' ? 1 : -1;
  return [...rows].sort((a, b) => {
    const va = get(a);
    const vb = get(b);
    if (typeof va === 'number' && typeof vb === 'number') return (va - vb) * dir;
    return String(va ?? '').localeCompare(String(vb ?? ''), 'ar') * dir;
  });
});

const pageCount = computed(() => (props.pageSize ? Math.max(1, Math.ceil(sorted.value.length / props.pageSize)) : 1));
const visible = computed(() =>
  props.pageSize ? sorted.value.slice((page.value - 1) * props.pageSize, page.value * props.pageSize) : sorted.value,
);

watch(() => props.rows, () => (page.value = 1));

function toggleSort(col: Column<T>) {
  if (!col.sortable) return;
  if (sortKey.value === col.key) {
    if (sortDir.value === 'asc') sortDir.value = 'desc';
    else sortKey.value = null;
  } else {
    sortKey.value = col.key;
    sortDir.value = 'asc';
  }
}

function alignClass(col: Column<T>) {
  if (col.align === 'center') return 'text-center';
  if (col.align === 'end') return 'text-end';
  return 'text-start';
}
</script>

<template>
  <div class="overflow-hidden rounded-xl border border-border">
    <div class="overflow-x-auto">
      <table class="w-full border-collapse text-body">
        <thead :class="sticky && 'sticky top-0 z-10'">
          <tr class="bg-surface">
            <th
              v-for="col in columns"
              :key="col.key"
              scope="col"
              class="whitespace-nowrap border-b border-border px-3 py-2.5 text-xs font-medium text-text-secondary"
              :class="[alignClass(col), col.sortable && 'cursor-pointer select-none hover:text-text-primary', col.noPrint && 'no-print']"
              :style="col.width ? { width: col.width } : undefined"
              @click="toggleSort(col)"
            >
              <span class="inline-flex items-center gap-1">
                {{ col.label }}
                <template v-if="sortKey === col.key">
                  <ArrowUp v-if="sortDir === 'asc'" class="size-3" />
                  <ArrowDown v-else class="size-3" />
                </template>
              </span>
            </th>
          </tr>
        </thead>
        <tbody v-if="loading && !rows?.length">
          <tr v-for="i in skeletonRows" :key="i" class="border-b border-border last:border-0">
            <td v-for="col in columns" :key="col.key" class="px-3 py-3">
              <div class="h-3.5 animate-shimmer rounded bg-surface-hover" :style="{ width: `${50 + ((i * 7 + col.key.length * 13) % 45)}%` }" />
            </td>
          </tr>
        </tbody>
        <tbody v-else-if="error">
          <tr>
            <td :colspan="columns.length"><ErrorState :message="error" compact @retry="emit('retry')" /></td>
          </tr>
        </tbody>
        <tbody v-else-if="!visible.length">
          <tr>
            <td :colspan="columns.length">
              <slot name="empty">
                <EmptyState :title="emptyTitle" :description="emptyDescription" :icon="emptyIcon" compact />
              </slot>
            </td>
          </tr>
        </tbody>
        <tbody v-else :class="loading && 'opacity-60 transition-opacity'">
          <tr
            v-for="row in visible"
            :key="row[rowKey]"
            class="border-b border-border transition-colors last:border-0"
            :class="[
              clickable && 'cursor-pointer hover:bg-surface-hover',
              highlightKey && row[rowKey] === highlightKey && 'bg-primary/8',
            ]"
            @click="clickable && emit('row-click', row)"
          >
            <td
              v-for="col in columns"
              :key="col.key"
              class="px-3 py-3 align-middle"
              :class="[alignClass(col), col.class, col.noPrint && 'no-print']"
            >
              <slot :name="`cell-${col.key}`" :row="row" :value="row[col.key]">
                <span v-if="col.numeric" class="num">{{ formatNumber(row[col.key]) }}</span>
                <template v-else>{{ row[col.key] ?? '—' }}</template>
              </slot>
            </td>
          </tr>
        </tbody>
        <tfoot v-if="$slots.footer && visible.length" class="border-t border-border bg-surface font-medium">
          <slot name="footer" />
        </tfoot>
      </table>
    </div>
    <div
      v-if="pageSize && pageCount > 1"
      class="no-print flex items-center justify-between border-t border-border bg-surface px-3 py-2 text-xs text-text-secondary"
    >
      <span>
        عرض <span class="num">{{ formatNumber((page - 1) * pageSize + 1) }}–{{ formatNumber(Math.min(page * pageSize, sorted.length)) }}</span>
        من <span class="num">{{ formatNumber(sorted.length) }}</span>
      </span>
      <div class="flex items-center gap-1">
        <button
          type="button"
          class="rounded-md p-1 hover:bg-surface-hover disabled:opacity-40"
          :disabled="page <= 1"
          aria-label="الصفحة السابقة"
          @click="page--"
        >
          <ChevronRight class="size-4" />
        </button>
        <span class="num min-w-12 text-center">{{ formatNumber(page) }} / {{ formatNumber(pageCount) }}</span>
        <button
          type="button"
          class="rounded-md p-1 hover:bg-surface-hover disabled:opacity-40"
          :disabled="page >= pageCount"
          aria-label="الصفحة التالية"
          @click="page++"
        >
          <ChevronLeft class="size-4" />
        </button>
      </div>
    </div>
  </div>
</template>
