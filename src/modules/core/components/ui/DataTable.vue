<script setup lang="ts" generic="T extends Record<string, any>">
import { computed, ref, shallowRef, watch, type Component } from 'vue';
import { ArrowDown, ArrowUp, Download, LoaderCircle } from '@lucide/vue';
import { rowsPerPage as settingsRowsPerPage, zebraRows } from '../../controllers/useAppearance';
import { exportXlsx, type ExportColumn } from '../../helpers/exportXlsx';
import { dirIcon } from '../../helpers/dirIcon';
import { formatNumber } from '../../helpers/format';
import DirIcon from './DirIcon.vue';
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell, TableFooter } from '@/modules/core/components/shadcn/table';
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

export interface ServerPageQuery<F = any> {
  page: number;
  pageSize: number;
  sort: { key: string; dir: 'asc' | 'desc' } | null;
  filters?: F;
}

export interface ServerPageResult<R> {
  rows: R[];
  total: number;
  totals?: Record<string, number>;
}

const props = withDefaults(
  defineProps<{
    columns: Column<T>[];
    /** Plain-array (client) mode: the full row set, sorted/paginated in the browser. Ignored when `fetchPage` is set. */
    rows?: T[] | undefined;
    rowKey?: string;
    loading?: boolean;
    error?: string | null;
    clickable?: boolean;
    /** 0 disables paging. Omitted uses the appearance setting (rows per page: 25/50/100). */
    pageSize?: number;
    emptyTitle?: string;
    emptyDescription?: string;
    emptyIcon?: Component;
    /** Highlight a row (e.g. just-created record). */
    highlightKey?: string;
    skeletonRows?: number;
    sticky?: boolean;
    /**
     * Server mode: when provided, the table calls this instead of using `rows` + client-side
     * sort/paginate. `filters` (any shape the caller's service expects) is passed through
     * unchanged on every call; the table re-calls whenever page/sort/`filters` change.
     */
    fetchPage?: (query: ServerPageQuery) => Promise<ServerPageResult<T>>;
    filters?: any;
    /**
     * File name (without extension) that enables the "تصدير" export button. Exports every
     * filtered row — in server mode this pages through `fetchPage` (ignoring the on-screen page),
     * in plain mode it uses the full `rows` array, not just the current page.
     */
    exportFileName?: string;
    /** Overrides which columns/labels go into the export; defaults to `columns` minus `noPrint` ones. */
    exportColumns?: ExportColumn<T>[];
  }>(),
  { rowKey: 'id', skeletonRows: 8, emptyTitle: 'لا توجد سجلات' },
);

const emit = defineEmits<{ 'row-click': [row: T]; retry: [] }>();

const isServerMode = computed(() => !!props.fetchPage);

// `pageSize={number}` (including 0, to disable paging) always wins; omitted follows the appearance
// setting so changing "rows per page" in Settings → المظهر affects every table that didn't ask
// for a specific size.
const effectivePageSize = computed(() => (props.pageSize !== undefined ? props.pageSize : settingsRowsPerPage.value));

const sortKey = ref<string | null>(null);
const sortDir = ref<'asc' | 'desc'>('asc');
const page = ref(1);

// --- Client (plain-array) mode -----------------------------------------------------------------

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

const clientPageCount = computed(() => (effectivePageSize.value ? Math.max(1, Math.ceil(sorted.value.length / effectivePageSize.value)) : 1));
const clientVisible = computed(() =>
  effectivePageSize.value ? sorted.value.slice((page.value - 1) * effectivePageSize.value, page.value * effectivePageSize.value) : sorted.value,
);

watch(() => props.rows, () => {
  if (!isServerMode.value) page.value = 1;
});

// --- Server mode ----------------------------------------------------------------------------

const serverRows = shallowRef<T[]>([]);
const serverTotal = ref(0);
const serverTotals = ref<Record<string, number> | undefined>();
const serverLoading = ref(false);
const serverError = ref<string | null>(null);
let requestId = 0;

async function loadServerPage() {
  if (!props.fetchPage) return;
  const id = ++requestId;
  serverLoading.value = true;
  serverError.value = null;
  try {
    const result = await props.fetchPage({
      page: page.value,
      pageSize: effectivePageSize.value || 25,
      sort: sortKey.value ? { key: sortKey.value, dir: sortDir.value } : null,
      filters: props.filters,
    });
    if (id !== requestId) return;
    serverRows.value = result.rows;
    serverTotal.value = result.total;
    serverTotals.value = result.totals;
  } catch (err) {
    if (id !== requestId) return;
    serverError.value = err instanceof Error ? err.message : 'تعذر تحميل البيانات';
  } finally {
    if (id === requestId) serverLoading.value = false;
  }
}

watch(
  () => [props.filters, sortKey.value, sortDir.value, effectivePageSize.value],
  () => {
    if (isServerMode.value) {
      page.value = 1;
      void loadServerPage();
    } else {
      page.value = 1;
    }
  },
  { deep: true },
);
watch(page, () => {
  if (isServerMode.value) void loadServerPage();
});
watch(
  () => props.fetchPage,
  (fn) => {
    if (fn) void loadServerPage();
  },
  { immediate: true },
);

defineExpose({ reload: loadServerPage, serverTotals });

// --- Combined (mode-agnostic) view used by the template ---------------------------------------

const visible = computed(() => (isServerMode.value ? serverRows.value : clientVisible.value));
const totalRowCount = computed(() => (isServerMode.value ? serverTotal.value : sorted.value.length));
const pageCount = computed(() =>
  isServerMode.value ? (effectivePageSize.value ? Math.max(1, Math.ceil(serverTotal.value / effectivePageSize.value)) : 1) : clientPageCount.value,
);
const effectiveLoading = computed(() => (isServerMode.value ? serverLoading.value : props.loading));
const effectiveError = computed(() => (isServerMode.value ? serverError.value : props.error));

function retry() {
  if (isServerMode.value) void loadServerPage();
  else emit('retry');
}

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

function onRowClick(row: T) {
  if (props.clickable) emit('row-click', row);
}

function alignClass(col: Column<T>) {
  if (col.align === 'center') return 'text-center';
  if (col.align === 'end') return 'text-end';
  return 'text-start';
}

// --- Excel export -------------------------------------------------------------------------------

const exporting = ref(false);

// Columns with a `sortValue` (computed/derived cells like a resolved name or a percentage that
// isn't stored directly on the row) export through it too, so the export matches what's on screen
// rather than a raw id or `undefined`.
const defaultExportColumns = computed<ExportColumn<T>[]>(() =>
  props.columns
    .filter((c) => !c.noPrint)
    .map((c) => ({ key: c.key, label: c.label, numeric: c.numeric, value: c.sortValue })),
);

async function exportRows() {
  if (!props.exportFileName || exporting.value) return;
  exporting.value = true;
  try {
    await exportXlsx<T>({
      fileName: props.exportFileName,
      columns: props.exportColumns ?? defaultExportColumns.value,
      ...(isServerMode.value
        ? {
            fetchAll: async ({ page, pageSize }) => {
              const result = await props.fetchPage!({ page, pageSize, sort: sortKey.value ? { key: sortKey.value, dir: sortDir.value } : null, filters: props.filters });
              return { rows: result.rows, total: result.total };
            },
          }
        : { rows: sorted.value }),
    });
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <div class="overflow-hidden rounded-xl border border-border">
    <div v-if="exportFileName" class="no-print flex items-center justify-end border-b border-border bg-surface px-3 py-1.5">
      <button
        type="button"
        class="inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-text-secondary hover:bg-surface-hover hover:text-text-primary disabled:opacity-50"
        :disabled="exporting"
        @click="exportRows"
      >
        <LoaderCircle v-if="exporting" class="size-3.5 animate-spin" />
        <Download v-else class="size-3.5" />
        تصدير
      </button>
    </div>
    <Table class="w-full border-collapse text-body" container-class="overflow-x-auto">
      <TableHeader :class="sticky && 'sticky top-0 z-10'">
        <TableRow class="bg-surface hover:bg-surface">
          <TableHead
            v-for="col in columns"
            :key="col.key"
            scope="col"
            class="h-auto whitespace-nowrap border-b border-border px-3 py-2.5 text-xs font-medium text-text-secondary"
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
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody v-if="effectiveLoading && !visible.length">
        <TableRow v-for="i in skeletonRows" :key="i" class="border-b border-border last:border-0">
          <TableCell v-for="col in columns" :key="col.key" class="px-3 py-3">
            <div class="h-3.5 animate-shimmer rounded bg-surface-hover" :style="{ width: `${50 + ((i * 7 + col.key.length * 13) % 45)}%` }" />
          </TableCell>
        </TableRow>
      </TableBody>
      <TableBody v-else-if="effectiveError">
        <TableRow>
          <TableCell :colspan="columns.length" class="whitespace-normal"><ErrorState :message="effectiveError" compact @retry="retry" /></TableCell>
        </TableRow>
      </TableBody>
      <TableBody v-else-if="!visible.length">
        <TableRow>
          <TableCell :colspan="columns.length" class="whitespace-normal">
            <slot name="empty">
              <EmptyState :title="emptyTitle" :description="emptyDescription" :icon="emptyIcon" compact />
            </slot>
          </TableCell>
        </TableRow>
      </TableBody>
      <TableBody v-else :class="effectiveLoading && 'opacity-60 transition-opacity'">
        <TableRow
          v-for="(row, i) in visible"
          :key="row[rowKey]"
          class="border-b border-border transition-colors last:border-0"
          :style="{ height: 'var(--density-row-h)' }"
          :class="[
            clickable && 'cursor-pointer hover:bg-surface-hover',
            highlightKey && row[rowKey] === highlightKey && 'bg-primary/8',
            zebraRows && i % 2 === 1 && !(highlightKey && row[rowKey] === highlightKey) && 'bg-surface/60',
          ]"
          @click="onRowClick(row)"
        >
          <TableCell
            v-for="col in columns"
            :key="col.key"
            class="px-3 align-middle"
            :style="{ paddingBlock: 'var(--density-padding)' }"
            :class="[alignClass(col), col.class, col.noPrint && 'no-print']"
          >
            <slot :name="`cell-${col.key}`" :row="row" :value="row[col.key]">
              <span v-if="col.numeric" class="num">{{ formatNumber(row[col.key]) }}</span>
              <template v-else>{{ row[col.key] ?? '—' }}</template>
            </slot>
          </TableCell>
        </TableRow>
      </TableBody>
      <TableFooter v-if="$slots.footer && visible.length" class="border-t border-border bg-surface font-medium">
        <slot name="footer" />
      </TableFooter>
    </Table>
    <div
      v-if="effectivePageSize && pageCount > 1"
      class="no-print flex items-center justify-between border-t border-border bg-surface px-3 py-2 text-xs text-text-secondary"
    >
      <span>
        عرض <span class="num">{{ formatNumber((page - 1) * effectivePageSize + 1) }}–{{ formatNumber(Math.min(page * effectivePageSize, totalRowCount)) }}</span>
        من <span class="num">{{ formatNumber(totalRowCount) }}</span>
      </span>
      <div class="flex items-center gap-1">
        <button
          type="button"
          class="rounded-md p-1 hover:bg-surface-hover disabled:opacity-40"
          :disabled="page <= 1"
          aria-label="الصفحة السابقة"
          @click="page--"
        >
          <DirIcon :icon="dirIcon.prev" class="size-4" />
        </button>
        <span class="num min-w-12 text-center">{{ formatNumber(page) }} / {{ formatNumber(pageCount) }}</span>
        <button
          type="button"
          class="rounded-md p-1 hover:bg-surface-hover disabled:opacity-40"
          :disabled="page >= pageCount"
          aria-label="الصفحة التالية"
          @click="page++"
        >
          <DirIcon :icon="dirIcon.next" class="size-4" />
        </button>
      </div>
    </div>
  </div>
</template>
