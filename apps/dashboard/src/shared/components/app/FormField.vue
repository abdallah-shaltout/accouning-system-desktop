<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import { Field } from "vee-validate";
import { FormControl, FormItem, FormLabel, FormMessage } from "@/shared/components/ui/form";
import { Input } from "@/shared/components/ui/input";

/**
 * The project's form-field wrapper (CLAUDE.md UI rule "No bare `<input>`/`<select>`/`<label>` in
 * pages — use FormField + an App* control"). Wraps a vee-validate `Field` bound by `name` to the
 * page's form schema (see 05-ui-rules.md "Forms"), with a label and an error slot beneath it. By
 * default it renders a shadcn `Input`; pass a `control` slot to swap in a select, textarea, etc. and
 * still get the label/error wiring for free.
 */
interface Props {
  name: string;
  label?: string;
  type?: string;
  placeholder?: string;
  autocomplete?: string;
  class?: HTMLAttributes["class"];
  /** RTL rule: phone numbers, codes and other LTR-content fields (05-ui-rules.md "RTL"). */
  dir?: "ltr" | "rtl";
}

const props = defineProps<Props>();
</script>

<template>
  <Field v-slot="{ field, errorMessage }" :name="name">
    <FormItem :class="props.class">
      <FormLabel v-if="label">{{ label }}</FormLabel>
      <FormControl>
        <slot name="control" :field="field" :error="errorMessage">
          <Input
            v-bind="field"
            :type="type ?? 'text'"
            :placeholder="placeholder"
            :autocomplete="autocomplete"
            :dir="dir"
            :aria-invalid="!!errorMessage"
          />
        </slot>
      </FormControl>
      <FormMessage />
    </FormItem>
  </Field>
</template>
