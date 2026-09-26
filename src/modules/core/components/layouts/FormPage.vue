<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 (F1 layouts table — "FormPage"): header · `FormSection`s (default slot) ·
 * aside (totals/help, `#aside` slot) · `FormActions` (`#actions` slot). Two-column on wide screens
 * (form + aside), single column on narrow ones — the aside stacks below the form.
 *
 * Not wired into any existing form page yet (F-2/F-3 migrate invoice/purchase/journal/product/party/…
 * forms — out of scope for F-0); this is the shell those pages will assemble once that batch starts.
 */
import type { RouteLocationRaw } from 'vue-router';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';

defineProps<{
  title: string;
  subtitle?: string;
  back?: RouteLocationRaw;
}>();
</script>

<template>
  <div>
    <PageHeader :title="title" :subtitle="subtitle" :back="back">
      <template #actions><slot name="header-actions" /></template>
    </PageHeader>

    <div class="grid grid-cols-1 gap-6 lg:grid-cols-[1fr_320px]">
      <div class="space-y-4">
        <slot />
      </div>
      <aside v-if="$slots.aside" class="space-y-4">
        <slot name="aside" />
      </aside>
    </div>

    <slot name="actions" />
  </div>
</template>
