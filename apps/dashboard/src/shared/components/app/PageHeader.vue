<script setup lang="ts">
import { ArrowRight } from "@lucide/vue";
import { RouterLink } from "vue-router";
import type { AppRoute } from "@/shared/types/route";

/**
 * Page title + optional back link + optional primary-action slot (CLAUDE.md "Build from
 * shared/components/app/*"). The back icon points → per the RTL rule ("back points →").
 */
interface Props {
  title: string;
  subtitle?: string;
  back?: AppRoute;
}

defineProps<Props>();
</script>

<template>
  <header class="flex flex-wrap items-start justify-between gap-3 pb-4">
    <div class="flex items-start gap-2">
      <RouterLink
        v-if="back"
        :to="back"
        class="mt-1 flex size-7 shrink-0 items-center justify-center rounded-md text-text-secondary transition-colors hover:bg-surface-hover hover:text-text-primary"
        aria-label="رجوع"
      >
        <ArrowRight class="size-4" />
      </RouterLink>
      <div>
        <h1 class="text-heading font-semibold text-text-primary">{{ title }}</h1>
        <p v-if="subtitle" class="mt-1 text-body text-text-secondary">{{ subtitle }}</p>
      </div>
    </div>
    <div v-if="$slots.actions" class="flex shrink-0 items-center gap-2">
      <slot name="actions" />
    </div>
  </header>
</template>
