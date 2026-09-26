<script setup lang="ts" generic="V extends string | number">
/** Rebuilt on shadcn's NativeSelect (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — same props as
 * before. Deliberately kept native (not Combobox): fast, keyboard-friendly for dense data entry, and
 * keeps e2e's `locator("select")` selectors working. Our own `.control` styling, not shadcn's. */
import { computed, useId } from 'vue';
import { NativeSelect } from '@/modules/core/components/shadcn/native-select';

const props = defineProps<{
  label?: string;
  options: { value: V; label: string; disabled?: boolean }[];
  placeholder?: string;
  error?: string;
  hint?: string;
  required?: boolean;
  disabled?: boolean;
}>();

const model = defineModel<V | undefined | ''>();
const id = useId();
const errorId = computed(() => (props.error ? `${id}-error` : undefined));
</script>

<template>
  <div>
    <label v-if="label" :for="id" class="field-label">
      {{ label }}<span v-if="required" class="text-danger" aria-hidden="true"> *</span>
    </label>
    <NativeSelect
      :id="id"
      v-model="model"
      :disabled="disabled"
      :aria-invalid="!!error || undefined"
      :aria-describedby="errorId"
      :aria-required="required || undefined"
      class="control w-full rounded-md text-body shadow-none"
      :class="model === '' || model === undefined ? 'text-text-secondary' : ''"
    >
      <option v-if="placeholder !== undefined" value="">{{ placeholder }}</option>
      <option v-for="o in options" :key="String(o.value)" :value="o.value" :disabled="o.disabled" class="text-text-primary">
        {{ o.label }}
      </option>
    </NativeSelect>
    <p v-if="error" :id="errorId" class="mt-1 text-xs text-danger" role="alert">{{ error }}</p>
    <p v-else-if="hint" class="mt-1 text-xs text-text-secondary">{{ hint }}</p>
  </div>
</template>
