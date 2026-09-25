<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router';
import DirIcon from '@/modules/core/components/ui/DirIcon.vue';
import { dirIcon } from '@/modules/core/helpers/dirIcon';

defineProps<{ title: string; subtitle?: string; back?: RouteLocationRaw }>();
</script>

<template>
  <div class="mb-5 flex flex-wrap items-start justify-between gap-3">
    <div class="flex min-w-0 items-start gap-2">
      <RouterLink
        v-if="back"
        :to="back"
        class="no-print mt-0.5 rounded-md p-1 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
        aria-label="رجوع"
      >
        <DirIcon :icon="dirIcon.back" class="size-4" />
      </RouterLink>
      <div class="min-w-0">
        <h1 class="flex flex-wrap items-center gap-2 text-lg font-semibold tracking-tight">
          {{ title }}
          <slot name="badge" />
        </h1>
        <p v-if="subtitle || $slots.subtitle" class="mt-0.5 text-body text-text-secondary">
          <slot name="subtitle">{{ subtitle }}</slot>
        </p>
      </div>
    </div>
    <div v-if="$slots.actions" class="no-print flex flex-wrap items-center gap-2">
      <slot name="actions" />
    </div>
  </div>
</template>
