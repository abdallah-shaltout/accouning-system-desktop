<script setup lang="ts">
/**
 * Password field used by every password/credential input in the app. Wraps `AppInput` and:
 * - forces LTR (passwords are a code-like value, same family as phone numbers/IBANs — rule 19),
 * - suppresses the browser's own reveal-eye (Edge/IE `::-ms-reveal`, `::-ms-clear`) so only our
 *   toggle button controls visibility,
 * - toggles the native `type` between `password`/`text` rather than faking it with CSS, so the
 *   platform's password-manager and autofill heuristics keep working.
 * - left-aligns the typed dots/text (overrides AppInput's default text-end for ltr fields — see
 *   `passwordInputClass` below).
 */
import { computed, ref } from 'vue';
import { Eye, EyeOff } from '@lucide/vue';
import { cn } from '@/modules/core/helpers/utils';
import AppInput from './AppInput.vue';

const props = withDefaults(
  defineProps<{
    label?: string;
    placeholder?: string;
    error?: string;
    hint?: string;
    required?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    autofocus?: boolean;
    /** Passed through to the underlying input's autocomplete attribute (e.g. "current-password", "new-password"). */
    autocomplete?: string;
    inputClass?: string;
  }>(),
  {},
);

const model = defineModel<string | undefined | null>();
const input = ref<InstanceType<typeof AppInput>>();
const revealed = ref(false);

// Password dots are a left-anchored code-like value (rule 19), not reading-direction text —
// text-left deliberately overrides AppInput's default text-end for an `ltr` field.
const passwordInputClass = computed(() =>
  cn(/* rtl-ok: password dots are a left-anchored code-like value, not reading-direction text */ 'app-password-input text-left', props.inputClass),
);

defineExpose({
  focus: () => input.value?.focus(),
  select: () => input.value?.select(),
});
</script>

<template>
  <AppInput
    ref="input"
    v-model="model"
    :label="label"
    :type="revealed ? 'text' : 'password'"
    ltr
    :placeholder="placeholder"
    :error="error"
    :hint="hint"
    :required="required"
    :disabled="disabled"
    :readonly="readonly"
    :autofocus="autofocus"
    :autocomplete="autocomplete"
    :input-class="passwordInputClass"
  >
    <template #suffix>
      <button
        type="button"
        tabindex="-1"
        class="flex items-center text-text-secondary transition-colors hover:text-text-primary"
        :aria-label="revealed ? 'إخفاء كلمة المرور' : 'إظهار كلمة المرور'"
        :aria-pressed="revealed"
        :disabled="disabled"
        @click="revealed = !revealed"
      >
        <EyeOff v-if="revealed" class="size-4" />
        <Eye v-else class="size-4" />
      </button>
    </template>
  </AppInput>
</template>

<style>
/* Suppress every browser's built-in password reveal/clear control — this component supplies
   its own toggle, and a native one on top of ours would show two overlapping eye icons. */
.app-password-input::-ms-reveal,
.app-password-input::-ms-clear {
  display: none;
}
</style>
