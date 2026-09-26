<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 — the shared filter row for list pages: `SearchInput` + a set of declared
 * selects/segments (`filters`) + an optional `DateRangeFilter` + saved views + a "مسح" (clear) button.
 * Syncs every value to the URL query so a filtered list survives navigation/refresh (F1 "FilterBar"
 * row in the doc). Not wired into any existing list page yet — that's F-1, out of scope for F-0.
 */
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { X } from '@lucide/vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';

export interface FilterBarSelect {
  key: string;
  label?: string;
  placeholder?: string;
  options: { value: string; label: string }[];
}

export interface SavedView {
  key: string;
  label: string;
  query: Record<string, string>;
}

const props = withDefaults(
  defineProps<{
    /** URL query key for the search box. */
    searchKey?: string;
    searchPlaceholder?: string;
    /** Declared select/segment filters, synced to the URL by their `key`. */
    filters?: FilterBarSelect[];
    /** Show a date range filter synced to `from`/`to` query keys. */
    dateRange?: boolean;
    fiscalStart?: string;
    savedViews?: SavedView[];
  }>(),
  { searchKey: 'q', filters: () => [] },
);

const route = useRoute();
const router = useRouter();

function queryString(key: string): string {
  const v = route.query[key];
  return typeof v === 'string' ? v : '';
}

function setQuery(patch: Record<string, string | undefined>) {
  const next = { ...route.query, ...patch };
  for (const k of Object.keys(next)) if (!next[k]) delete next[k];
  void router.replace({ query: next });
}

const search = computed({
  get: () => queryString(props.searchKey),
  set: (v: string) => setQuery({ [props.searchKey]: v || undefined }),
});

function filterValue(key: string): string {
  return queryString(key);
}
function setFilter(key: string, v: string) {
  setQuery({ [key]: v || undefined });
}

const from = computed({ get: () => queryString('from'), set: (v: string) => setQuery({ from: v || undefined }) });
const to = computed({ get: () => queryString('to'), set: (v: string) => setQuery({ to: v || undefined }) });

const hasActiveFilters = computed(() => {
  if (search.value) return true;
  if (props.dateRange && (from.value || to.value)) return true;
  return props.filters.some((f) => filterValue(f.key));
});

function clearAll() {
  const cleared: Record<string, string | undefined> = { [props.searchKey]: undefined };
  for (const f of props.filters) cleared[f.key] = undefined;
  if (props.dateRange) {
    cleared.from = undefined;
    cleared.to = undefined;
  }
  setQuery(cleared);
}

function applyView(view: SavedView) {
  void router.replace({ query: { ...view.query } });
}
</script>

<template>
  <div class="no-print mb-4 flex flex-wrap items-end gap-3 rounded-xl border border-border bg-surface p-3">
    <SearchInput v-model="search" :placeholder="searchPlaceholder" />

    <div v-for="f in filters" :key="f.key" class="w-40">
      <AppSelect
        :model-value="filterValue(f.key)"
        :label="f.label"
        :placeholder="f.placeholder ?? 'الكل'"
        :options="f.options"
        @update:model-value="(v) => setFilter(f.key, String(v ?? ''))"
      />
    </div>

    <DateRangeFilter v-if="dateRange" v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />

    <div v-if="savedViews?.length" class="flex flex-wrap gap-1.5">
      <button
        v-for="v in savedViews"
        :key="v.key"
        type="button"
        class="h-[30px] rounded-full border border-border px-2.5 text-xs text-text-secondary transition-colors hover:bg-surface-hover hover:text-text-primary"
        @click="applyView(v)"
      >
        {{ v.label }}
      </button>
    </div>

    <AppButton v-if="hasActiveFilters" size="sm" variant="ghost" :icon="X" @click="clearAll">مسح</AppButton>
  </div>
</template>
