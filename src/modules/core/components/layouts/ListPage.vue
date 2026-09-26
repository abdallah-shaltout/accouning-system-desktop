<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 (F1 layouts table — "ListPage"): header (title + primary action) · `FilterBar`
 * · `DataTable` (passed via the default slot, since its generic row type can't be forwarded through
 * a wrapping component's slot type) · pagination (owned by `DataTable` itself already).
 *
 * Not wired into any existing list page yet (F-1 migrates invoices/products/customers/… — out of
 * scope for F-0); this is the shell those pages will assemble once that batch starts.
 */
import type { RouteLocationRaw } from 'vue-router';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import type { Component } from 'vue';

defineProps<{
  title: string;
  subtitle?: string;
  back?: RouteLocationRaw;
  primaryActionLabel?: string;
  primaryActionIcon?: Component;
  primaryActionTo?: RouteLocationRaw;
}>();
defineEmits<{ 'primary-action': [] }>();
</script>

<template>
  <div>
    <PageHeader :title="title" :subtitle="subtitle" :back="back">
      <template #actions>
        <slot name="actions" />
        <AppButton
          v-if="primaryActionLabel"
          variant="primary"
          :icon="primaryActionIcon"
          :to="primaryActionTo"
          @click="!primaryActionTo && $emit('primary-action')"
        >
          {{ primaryActionLabel }}
        </AppButton>
      </template>
    </PageHeader>

    <slot name="filters" />

    <slot />
  </div>
</template>
