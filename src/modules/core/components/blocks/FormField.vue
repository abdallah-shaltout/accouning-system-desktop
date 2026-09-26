<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 (`docs/v2/17-ui-system-rtl-themes.md` → "Phase F" → F1 blocks table): the one
 * shared label + hint + error wrapper every form field should use instead of hand-written
 * `<label>` + `<input>` + error `<p>` markup (CLAUDE.md UI rule 7 — "No bare `<input>`/`<select>`/
 * `<label>` in pages"). Wraps shadcn `Field` for structure/RTL and wires `aria-describedby` to the
 * hint/error ids so screen readers announce them. The control itself (an `App*` input, a custom
 * picker, anything) goes in the default slot — `FormField` never renders a native `<input>` itself.
 *
 * Not wired into any existing page yet — that migration is F-2/F-3, out of scope for F-0.
 */
import { computed, useId } from 'vue';
import { Field } from '@/modules/core/components/shadcn/field';

const props = withDefaults(
  defineProps<{
    label?: string;
    hint?: string;
    error?: string;
    required?: boolean;
    /** Field name — used to build stable ids when the slot control doesn't manage its own. */
    name?: string;
  }>(),
  {},
);

const autoId = useId();
const fieldId = computed(() => props.name ?? autoId);
const hintId = computed(() => `${fieldId.value}-hint`);
const errorId = computed(() => `${fieldId.value}-error`);
const describedBy = computed(() => [props.error ? errorId.value : null, !props.error && props.hint ? hintId.value : null].filter(Boolean).join(' ') || undefined);

defineExpose({ fieldId, describedBy });
</script>

<template>
  <Field :data-invalid="!!error || undefined" class="gap-1.5">
    <label v-if="label" :for="fieldId" class="field-label">
      {{ label }}<span v-if="required" class="text-danger"> *</span>
    </label>
    <slot :field-id="fieldId" :described-by="describedBy" :invalid="!!error" />
    <p v-if="error" :id="errorId" class="text-xs text-danger" role="alert">{{ error }}</p>
    <p v-else-if="hint" :id="hintId" class="text-xs text-text-secondary">{{ hint }}</p>
  </Field>
</template>
