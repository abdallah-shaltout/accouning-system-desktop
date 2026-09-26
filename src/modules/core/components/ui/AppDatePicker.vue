<script setup lang="ts">
/**
 * Single-date field on shadcn Popover + Calendar (plans/pending/19-shadcn-date-picker), replacing
 * native `<input type="date">` so the picker matches every other themed control (CLAUDE.md rule 13).
 * Model stays a plain `YYYY-MM-DD` string (or undefined) — the same contract `AppInput type="date"`
 * had — so callers, Zod validators and mock services never see `@internationalized/date`'s
 * `CalendarDate`. Conversion happens only at this component's own boundary, via `CalendarDate`'s
 * year/month/day fields (no `Date` object round-trip), to avoid timezone drift.
 */
import { computed, ref, useId, watch } from 'vue';
import { CalendarDate, type DateValue } from '@internationalized/date';
import { CalendarDays } from '@lucide/vue';
import { Popover, PopoverContent, PopoverTrigger } from '@/modules/core/components/shadcn/popover';
import { Calendar } from '@/modules/core/components/shadcn/calendar';

const props = withDefaults(
  defineProps<{
    label?: string;
    placeholder?: string;
    error?: string;
    hint?: string;
    required?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    min?: string;
    max?: string;
    /** Compact table-cell variant (line-item expiry dates) — smaller height, no label row assumed. */
    compact?: boolean;
  }>(),
  {},
);

const model = defineModel<string | undefined>();
const id = useId();
const open = ref(false);
const textInput = ref<HTMLInputElement>();

/** `YYYY-MM-DD` — the exact shape `dateKeyToIso()` (core/helpers/format.ts) expects. */
const DATE_KEY = /^(\d{4})-(\d{2})-(\d{2})$/;

function isValidKey(key: string): boolean {
  const m = DATE_KEY.exec(key);
  if (!m) return false;
  const [, y, mo, d] = m;
  const date = new Date(Number(y), Number(mo) - 1, Number(d));
  return date.getFullYear() === Number(y) && date.getMonth() === Number(mo) - 1 && date.getDate() === Number(d);
}

function keyToCalendarDate(key: string | undefined): CalendarDate | undefined {
  if (!key || !isValidKey(key)) return undefined;
  const [, y, mo, d] = DATE_KEY.exec(key)!;
  return new CalendarDate(Number(y), Number(mo), Number(d));
}

function calendarDateToKey(value: DateValue): string {
  return `${String(value.year).padStart(4, '0')}-${String(value.month).padStart(2, '0')}-${String(value.day).padStart(2, '0')}`;
}

// Text the field shows while typing — kept separate from the model so an in-progress keystroke
// (e.g. "2026-0") isn't rejected mid-edit; only committed (blur) values validate against the model.
const text = ref(model.value ?? '');
watch(model, (value) => {
  if (value !== text.value) text.value = value ?? '';
});

const calendarValue = computed<DateValue | undefined>({
  get: () => keyToCalendarDate(model.value),
  set: (value) => {
    if (!value) return;
    model.value = calendarDateToKey(value);
    text.value = model.value;
    open.value = false;
  },
});

const minValue = computed(() => keyToCalendarDate(props.min));
const maxValue = computed(() => keyToCalendarDate(props.max));

function onTextInput(e: Event) {
  text.value = (e.target as HTMLInputElement).value;
}

function onBlur() {
  const trimmed = text.value.trim();
  if (trimmed === '') {
    // Empty stays '' (not undefined) — matches native <input type="date">'s cleared value, which
    // every existing `ref<string>`/`reactive` field this component replaces already expects.
    model.value = '';
    text.value = '';
    return;
  }
  if (isValidKey(trimmed)) {
    model.value = trimmed;
    text.value = trimmed;
  } else {
    // Invalid typed text reverts to the last valid model value — never silently coerced to today.
    text.value = model.value ?? '';
  }
}

defineExpose({
  focus: () => textInput.value?.focus(),
});
</script>

<template>
  <div>
    <label v-if="label" :for="id" class="field-label">
      {{ label }}<span v-if="required" class="text-danger"> *</span>
    </label>
    <Popover v-model:open="open">
      <div class="relative flex items-center">
        <input
          :id="id"
          ref="textInput"
          :value="text"
          type="text"
          dir="ltr"
          :placeholder="placeholder ?? 'YYYY-MM-DD'"
          :disabled="disabled"
          :readonly="readonly"
          :aria-invalid="!!error || undefined"
          class="control text-end text-body shadow-none pe-9"
          :class="compact ? 'h-8' : 'h-[34px]'"
          @input="onTextInput"
          @blur="onBlur"
        />
        <PopoverTrigger as-child>
          <button
            v-if="!disabled && !readonly"
            type="button"
            class="absolute end-1.5 flex items-center rounded p-1 text-text-secondary transition-colors hover:bg-surface-hover hover:text-text-primary"
            aria-label="فتح التقويم"
          >
            <CalendarDays class="size-4" />
          </button>
        </PopoverTrigger>
      </div>
      <PopoverContent class="w-auto p-0" align="start">
        <Calendar v-model="calendarValue" layout="month-and-year" :min-value="minValue" :max-value="maxValue" />
      </PopoverContent>
    </Popover>
    <p v-if="error" class="mt-1 text-xs text-danger">{{ error }}</p>
    <p v-else-if="hint" class="mt-1 text-xs text-text-secondary">{{ hint }}</p>
  </div>
</template>
