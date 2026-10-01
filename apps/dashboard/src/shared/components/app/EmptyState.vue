<script setup lang="ts">
import type { Component } from "vue";
import { InboxIcon } from "@lucide/vue";
import { copy } from "@/shared/config/copy";

/** Icon + message + optional action slot, used by DataTable's empty state and standalone pages. */
interface Props {
  icon?: Component;
  message?: string;
}

withDefaults(defineProps<Props>(), {
  message: copy.empty.default,
});
</script>

<template>
  <div class="flex flex-col items-center justify-center gap-3 px-6 py-12 text-center">
    <component :is="icon ?? InboxIcon" class="size-8 text-text-secondary" />
    <p class="text-body text-text-secondary">{{ message }}</p>
    <div v-if="$slots.action">
      <slot name="action" />
    </div>
  </div>
</template>
