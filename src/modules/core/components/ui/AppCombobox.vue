<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, useId, watch } from 'vue';
import { Check, ChevronDown, Search, X } from '@lucide/vue';

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
const open = ref(false);
const query = ref('');
const active = ref(0);
const trigger = ref<HTMLButtonElement>();
const searchInput = ref<HTMLInputElement>();
const list = ref<HTMLUListElement>();
const panelStyle = ref<Record<string, string>>({});

const selected = computed(() => props.options.find((o) => o.value === model.value));

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  const items = q
    ? props.options.filter((o) => `${o.label} ${o.sublabel ?? ''} ${o.keywords ?? ''}`.toLowerCase().includes(q))
    : props.options;
  return items.slice(0, 200);
});

watch(filtered, () => (active.value = 0));

function position() {
  const rect = trigger.value?.getBoundingClientRect();
  if (!rect) return;
  const spaceBelow = window.innerHeight - rect.bottom;
  const height = 300;
  const above = spaceBelow < height && rect.top > spaceBelow;
  panelStyle.value = {
    position: 'fixed',
    width: `${Math.max(rect.width, 240)}px`,
    right: `${window.innerWidth - rect.right}px`,
    ...(above ? { bottom: `${window.innerHeight - rect.top + 4}px` } : { top: `${rect.bottom + 4}px` }),
  };
}

async function show() {
  if (props.disabled) return;
  open.value = true;
  query.value = '';
  position();
  const idx = filtered.value.findIndex((o) => o.value === model.value);
  active.value = Math.max(0, idx);
  await nextTick();
  searchInput.value?.focus();
  scrollActive();
  window.addEventListener('scroll', onViewportChange, true);
  window.addEventListener('resize', onViewportChange);
}

function hide(refocus = true) {
  open.value = false;
  window.removeEventListener('scroll', onViewportChange, true);
  window.removeEventListener('resize', onViewportChange);
  if (refocus) trigger.value?.focus();
}

function onViewportChange(e: Event) {
  // Scrolling inside the list itself must not close the panel.
  if (e.type === 'scroll' && list.value?.contains(e.target as Node)) return;
  hide(false);
}

function choose(option: ComboOption) {
  if (option.disabled) return;
  model.value = option.value;
  emit('select', option);
  hide();
}

function scrollActive() {
  nextTick(() => list.value?.querySelector<HTMLElement>(`[data-index="${active.value}"]`)?.scrollIntoView({ block: 'nearest' }));
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    active.value = Math.min(active.value + 1, filtered.value.length - 1);
    scrollActive();
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    active.value = Math.max(active.value - 1, 0);
    scrollActive();
  } else if (e.key === 'Enter') {
    e.preventDefault();
    const option = filtered.value[active.value];
    if (option) choose(option);
  } else if (e.key === 'Escape') {
    e.preventDefault();
    e.stopPropagation();
    hide();
  } else if (e.key === 'Tab') {
    hide(false);
  }
}

function onTriggerKeydown(e: KeyboardEvent) {
  if (['ArrowDown', 'Enter', ' '].includes(e.key)) {
    e.preventDefault();
    show();
  }
}

onBeforeUnmount(() => hide(false));

defineExpose({ open: show });
</script>

<template>
  <div>
    <label v-if="label" :for="id" class="field-label">
      {{ label }}<span v-if="required" class="text-danger"> *</span>
    </label>
    <div class="relative">
      <button
        :id="id"
        ref="trigger"
        type="button"
        :disabled="disabled"
        :aria-invalid="!!error || undefined"
        aria-haspopup="listbox"
        :aria-expanded="open"
        class="control flex items-center gap-2 text-start"
        :class="dense ? 'h-8 border-transparent bg-transparent hover:border-border' : ''"
        @click="open ? hide() : show()"
        @keydown="onTriggerKeydown"
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
    </div>
    <p v-if="error" class="mt-1 text-xs text-danger">{{ error }}</p>

    <Teleport to="body">
      <div v-if="open" class="fixed inset-0 z-[60]" @mousedown="hide(false)" />
      <div
        v-if="open"
        dir="rtl"
        :style="panelStyle"
        class="z-[61] overflow-hidden rounded-lg border border-border bg-background shadow-xl"
        @keydown="onKeydown"
      >
        <div class="flex items-center gap-2 border-b border-border px-2.5">
          <Search class="size-4 text-text-secondary" />
          <input
            ref="searchInput"
            v-model="query"
            :placeholder="searchPlaceholder"
            class="h-9 w-full bg-transparent text-[13px] outline-none placeholder:text-text-secondary"
          />
        </div>
        <ul ref="list" role="listbox" class="max-h-64 overflow-y-auto p-1">
          <li
            v-for="(o, i) in filtered"
            :key="o.value"
            role="option"
            :data-index="i"
            :aria-selected="o.value === model"
            class="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-[13px]"
            :class="[i === active ? 'bg-surface-hover' : '', o.disabled && 'cursor-not-allowed opacity-50']"
            @mouseenter="active = i"
            @mousedown.prevent="choose(o)"
          >
            <span class="min-w-0 flex-1">
              <span class="block truncate">{{ o.label }}</span>
              <span v-if="o.sublabel" class="block truncate text-xs text-text-secondary">{{ o.sublabel }}</span>
            </span>
            <Check v-if="o.value === model" class="size-4 text-primary" />
          </li>
          <li v-if="!filtered.length" class="px-2 py-6 text-center text-xs text-text-secondary">{{ emptyText }}</li>
        </ul>
      </div>
    </Teleport>
  </div>
</template>
