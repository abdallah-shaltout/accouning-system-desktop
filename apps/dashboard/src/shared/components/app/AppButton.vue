<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { Loader2Icon } from "@lucide/vue";
import { Button, type ButtonVariants } from "@/shared/components/ui/button";
import type { AppRoute } from "@/shared/types/route";

/**
 * The project's button wrapper (CLAUDE.md UI rule 1 — "shadcn first"). Adds a `loading` spinner state
 * and a `to` prop that only accepts a named route object (never a path string, rule 25) — when `to`
 * is set, the button renders as a `RouterLink`.
 */
interface Props {
  variant?: ButtonVariants["variant"];
  size?: ButtonVariants["size"];
  class?: HTMLAttributes["class"];
  loading?: boolean;
  disabled?: boolean;
  to?: AppRoute;
  type?: "button" | "submit" | "reset";
}

const props = withDefaults(defineProps<Props>(), {
  type: "button",
});

const isDisabled = computed(() => props.disabled || props.loading);
</script>

<template>
  <Button
    v-if="!to"
    :variant="variant"
    :size="size"
    :class="props.class"
    :disabled="isDisabled"
    :type="type"
    :aria-busy="loading"
  >
    <Loader2Icon v-if="loading" class="animate-spin" />
    <slot />
  </Button>
  <Button v-else :variant="variant" :size="size" :class="props.class" :disabled="isDisabled" as-child>
    <RouterLink :to="to">
      <slot />
    </RouterLink>
  </Button>
</template>
