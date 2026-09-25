<script setup lang="ts">
/** Rebuilt on shadcn's Input (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — same props/slots as
 * before. Keeps our own `.control` styling rather than switching to shadcn's utility classes, since
 * `.control` already matches this design system's tokens exactly and a plain `<input>` gains no
 * accessibility from the wrapper (unlike composite primitives such as Switch or Dialog). */
import { computed, ref, useId } from 'vue';
import { Input } from '@/modules/core/components/shadcn/input';

const props = withDefaults(
  defineProps<{
    label?: string;
    type?: 'text' | 'number' | 'password' | 'date' | 'email' | 'tel' | 'search';
    placeholder?: string;
    error?: string;
    hint?: string;
    required?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    min?: number | string;
    max?: number | string;
    step?: number | string;
    /** Render value LTR (codes, phone numbers, VAT numbers). Number inputs are always LTR. */
    ltr?: boolean;
    autofocus?: boolean;
    inputClass?: string;
  }>(),
  { type: 'text' },
);

const model = defineModel<string | number | undefined | null>();
const id = useId();
const input = ref<InstanceType<typeof Input>>();

const value = computed({
  get: () => model.value ?? '',
  set: (v: string | number) => {
    if (props.type === 'number') model.value = v === '' || v === null ? undefined : Number(v);
    else model.value = v as string;
  },
});

defineExpose({
  focus: () => (input.value?.$el as HTMLInputElement | undefined)?.focus(),
  select: () => (input.value?.$el as HTMLInputElement | undefined)?.select(),
});
</script>

<template>
  <div>
    <label v-if="label" :for="id" class="field-label">
      {{ label }}<span v-if="required" class="text-danger"> *</span>
    </label>
    <div class="relative flex items-center">
      <span v-if="$slots.prefix" class="pointer-events-none absolute start-2.5 flex items-center text-text-secondary">
        <slot name="prefix" />
      </span>
      <Input
        :id="id"
        ref="input"
        v-model="value"
        :type="type"
        :placeholder="placeholder"
        :disabled="disabled"
        :readonly="readonly"
        :min="min"
        :max="max"
        :step="step ?? (type === 'number' ? 'any' : undefined)"
        :autofocus="autofocus"
        :aria-invalid="!!error || undefined"
        :dir="ltr || type === 'number' || type === 'date' ? 'ltr' : undefined"
        class="control h-[34px] rounded-md text-body shadow-none"
        :class="[$slots.prefix && 'ps-8', $slots.suffix && 'pe-10', (ltr || type === 'number') && 'text-right', inputClass]"
      />
      <span v-if="$slots.suffix" class="absolute end-2.5 flex items-center text-xs text-text-secondary">
        <slot name="suffix" />
      </span>
    </div>
    <p v-if="error" class="mt-1 text-xs text-danger">{{ error }}</p>
    <p v-else-if="hint" class="mt-1 text-xs text-text-secondary">{{ hint }}</p>
  </div>
</template>
