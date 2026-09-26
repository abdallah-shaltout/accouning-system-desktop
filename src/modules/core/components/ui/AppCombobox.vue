<script setup lang="ts">
/**
 * Rebuilt on shadcn's Combobox, i.e. reka-ui's ComboboxRoot (docs/v2/16-equal-rebrand-and-ui-kit.md
 * Phase C) — same props/model/emits as before, `aria-haspopup=listbox` kept on the trigger for e2e.
 * Real upgrade over the hand-rolled ~200-line version: arrow-key nav, Escape/Tab handling, listbox
 * ARIA wiring and popper positioning all come from reka-ui now instead of manual keydown handlers
 * and a `getBoundingClientRect()` positioning calculation. `ignoreFilter` keeps our own
 * Arabic-normalized `matchesSearch()` filtering rather than reka-ui's plain substring match.
 */
import { computed, ref, useId } from 'vue';
import { Check, ChevronDown, X } from '@lucide/vue';
import {
  Combobox,
  ComboboxAnchor,
  ComboboxTrigger,
  ComboboxInput,
  ComboboxList,
  ComboboxViewport,
  ComboboxEmpty,
  ComboboxItem,
  ComboboxItemIndicator,
} from '@/modules/core/components/shadcn/combobox';
import { matchesSearch } from '../../helpers/search';

export interface ComboOption {
  value: string;
  label: string;
  sublabel?: string;
  /** Extra text matched by the search box but not displayed (codes, barcodes…). */
  keywords?: string;
  disabled?: boolean;
}

const props = withDefaults(
  defineProps<{
    options: ComboOption[];
    label?: string;
    placeholder?: string;
    searchPlaceholder?: string;
    error?: string;
    required?: boolean;
    disabled?: boolean;
    clearable?: boolean;
    emptyText?: string;
    /** Compact variant for table cells. */
    dense?: boolean;
  }>(),
  { placeholder: 'اختر…', searchPlaceholder: 'بحث…', emptyText: 'لا توجد نتائج' },
);

const model = defineModel<string | undefined>();
const emit = defineEmits<{ select: [option: ComboOption] }>();

const id = useId();
const errorId = computed(() => (props.error ? `${id}-error` : undefined));
const open = ref(false);
const query = ref('');

const selected = computed(() => props.options.find((o) => o.value === model.value));

const filtered = computed(() => {
  const items = query.value.trim()
    ? props.options.filter((o) => matchesSearch([o.label, o.sublabel, o.keywords], query.value))
    : props.options;
  return items.slice(0, 200);
});

function onUpdateOpen(v: boolean) {
  open.value = v;
  if (v) query.value = '';
}

function onUpdateModelValue(v: unknown) {
  const value = v as string | undefined;
  model.value = value;
  const option = props.options.find((o) => o.value === value);
  if (option) emit('select', option);
}

defineExpose({ open: () => (open.value = true) });
</script>

<template>
  <div>
    <label v-if="label" :for="id" class="field-label">
      {{ label }}<span v-if="required" class="text-danger" aria-hidden="true"> *</span>
    </label>
    <Combobox
      :model-value="model"
      :open="open"
      ignore-filter
      :disabled="disabled"
      @update:model-value="onUpdateModelValue"
      @update:open="onUpdateOpen"
    >
      <ComboboxAnchor class="w-full">
        <ComboboxTrigger as-child aria-haspopup="listbox" class="w-full">
          <button
            :id="id"
            type="button"
            :disabled="disabled"
            :aria-invalid="!!error || undefined"
            :aria-describedby="errorId"
            :aria-required="required || undefined"
            class="control flex w-full items-center gap-2 text-start"
            :class="dense ? 'h-8 border-transparent bg-transparent hover:border-border' : ''"
          >
            <span class="min-w-0 flex-1 truncate" :class="!selected && 'text-text-secondary'">
              {{ selected?.label ?? placeholder }}
            </span>
            <span
              v-if="clearable && selected && !disabled"
              role="button"
              aria-label="مسح الاختيار"
              class="rounded p-0.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
              @click.stop="model = undefined"
            >
              <X class="size-3.5" />
            </span>
            <ChevronDown class="size-4 shrink-0 text-text-secondary" />
          </button>
        </ComboboxTrigger>
      </ComboboxAnchor>
      <p v-if="error" :id="errorId" class="mt-1 text-xs text-danger" role="alert">{{ error }}</p>

      <ComboboxList dir="rtl" class="w-[--reka-combobox-trigger-width] min-w-60 rounded-lg border-border bg-background shadow-xl">
        <ComboboxInput v-model="query" :placeholder="searchPlaceholder" class="text-body" />
        <ComboboxViewport class="p-1">
          <ComboboxItem
            v-for="o in filtered"
            :key="o.value"
            :value="o.value"
            :disabled="o.disabled"
            class="rounded-md px-2 py-1.5 text-body data-[disabled]:cursor-not-allowed"
          >
            <span class="min-w-0 flex-1">
              <span class="block truncate">{{ o.label }}</span>
              <span v-if="o.sublabel" class="block truncate text-xs text-text-secondary">{{ o.sublabel }}</span>
            </span>
            <ComboboxItemIndicator>
              <Check class="size-4 text-primary" />
            </ComboboxItemIndicator>
          </ComboboxItem>
          <ComboboxEmpty class="text-xs text-text-secondary">{{ emptyText }}</ComboboxEmpty>
        </ComboboxViewport>
      </ComboboxList>
    </Combobox>
  </div>
</template>
