<script setup lang="ts" generic="TData">
import { computed } from "vue";
import {
  FlexRender,
  getCoreRowModel,
  useVueTable,
  type ColumnDef,
  type SortingState,
} from "@tanstack/vue-table";
import { ArrowDownIcon, ArrowUpIcon, ArrowUpDownIcon, SearchIcon } from "@lucide/vue";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/shared/components/ui/table";
import { Input } from "@/shared/components/ui/input";
import { Skeleton } from "@/shared/components/ui/skeleton";
import EmptyState from "./EmptyState.vue";
import { copy } from "@/shared/config/copy";

/**
 * Server-driven data table (05-ui-rules.md "Tables"): the page owns the query state and re-fetches
 * on every `@update:*` event — this component never paginates/sorts/filters client-side. Column
 * definitions follow TanStack Table's own `ColumnDef` shape so a page can reuse its cell renderers
 * across `DataTable` instances.
 */
interface Props {
  columns: ColumnDef<TData, unknown>[];
  rows: TData[];
  loading?: boolean;
  /** 1-based current page, matching the backend's `ApiFeatures` `page` query param. */
  page?: number;
  limit?: number;
  total?: number;
  totalPages?: number;
  search?: string;
  searchPlaceholder?: string;
  sort?: string;
  emptyMessage?: string;
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  page: 1,
  limit: 20,
  total: 0,
  totalPages: 0,
  search: "",
  searchPlaceholder: "بحث…",
  sort: undefined,
  emptyMessage: undefined,
});

const emit = defineEmits<{
  (e: "update:page", page: number): void;
  (e: "update:sort", sort: string): void;
  (e: "update:search", search: string): void;
}>();

const sorting = computed<SortingState>(() => {
  if (!props.sort) return [];
  const desc = props.sort.startsWith("-");
  const id = desc ? props.sort.slice(1) : props.sort;
  return [{ id, desc }];
});

const table = useVueTable({
  get data() {
    return props.rows;
  },
  get columns() {
    return props.columns;
  },
  getCoreRowModel: getCoreRowModel(),
  manualPagination: true,
  manualSorting: true,
  state: {
    get sorting() {
      return sorting.value;
    },
  },
});

function toggleSort(columnId: string): void {
  const current = sorting.value[0];
  if (current?.id === columnId && !current.desc) {
    emit("update:sort", `-${columnId}`);
  } else if (current?.id === columnId && current.desc) {
    emit("update:sort", columnId);
  } else {
    emit("update:sort", columnId);
  }
}

function sortIconFor(columnId: string) {
  const current = sorting.value[0];
  if (current?.id !== columnId) return ArrowUpDownIcon;
  return current.desc ? ArrowDownIcon : ArrowUpIcon;
}

const hasPrev = computed(() => props.page > 1);
const hasNext = computed(() => props.totalPages > 0 && props.page < props.totalPages);
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="flex items-center gap-2">
      <div class="relative max-w-sm flex-1">
        <SearchIcon class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-text-secondary" />
        <Input
          :model-value="search"
          class="ps-8"
          :placeholder="searchPlaceholder"
          @update:model-value="(value) => emit('update:search', String(value))"
        />
      </div>
      <div v-if="$slots.toolbar">
        <slot name="toolbar" />
      </div>
    </div>

    <div class="overflow-hidden rounded-lg border border-border">
      <Table>
        <TableHeader>
          <TableRow v-for="headerGroup in table.getHeaderGroups()" :key="headerGroup.id">
            <TableHead
              v-for="header in headerGroup.headers"
              :key="header.id"
              :class="header.column.getCanSort() ? 'cursor-pointer select-none' : undefined"
              @click="header.column.getCanSort() && toggleSort(header.column.id)"
            >
              <span class="inline-flex items-center gap-1">
                <FlexRender
                  v-if="!header.isPlaceholder"
                  :render="header.column.columnDef.header"
                  :props="header.getContext()"
                />
                <component :is="sortIconFor(header.column.id)" v-if="header.column.getCanSort()" class="size-3.5" />
              </span>
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <template v-if="loading">
            <TableRow v-for="i in limit > 8 ? 8 : limit" :key="`skeleton-${i}`">
              <TableCell v-for="col in columns" :key="String(col.id ?? Math.random())">
                <Skeleton class="h-5 w-full" />
              </TableCell>
            </TableRow>
          </template>
          <template v-else-if="table.getRowModel().rows.length === 0">
            <TableRow>
              <TableCell :colspan="columns.length" class="p-0">
                <EmptyState :message="emptyMessage ?? (search ? copy.empty.search : copy.empty.default)" />
              </TableCell>
            </TableRow>
          </template>
          <template v-else>
            <TableRow v-for="row in table.getRowModel().rows" :key="row.id">
              <TableCell v-for="cell in row.getVisibleCells()" :key="cell.id">
                <FlexRender :render="cell.column.columnDef.cell" :props="cell.getContext()" />
              </TableCell>
            </TableRow>
          </template>
        </TableBody>
      </Table>
    </div>

    <div v-if="!loading && total > 0" class="flex items-center justify-between text-tiny text-text-secondary">
      <span class="num">{{ total }} نتيجة</span>
      <div class="flex items-center gap-2">
        <button
          type="button"
          class="rounded-md border border-border px-2 py-1 disabled:opacity-50"
          :disabled="!hasPrev"
          @click="emit('update:page', page - 1)"
        >
          السابق
        </button>
        <span class="num">{{ page }} / {{ totalPages || 1 }}</span>
        <button
          type="button"
          class="rounded-md border border-border px-2 py-1 disabled:opacity-50"
          :disabled="!hasNext"
          @click="emit('update:page', page + 1)"
        >
          التالي
        </button>
      </div>
    </div>
  </div>
</template>
