<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 — the one shared sticky save/cancel bar every form page should use instead of
 * a hand-rolled footer row. Shows the shared "تغييرات غير محفوظة" (unsaved changes) copy so it isn't
 * repeated per page (CLAUDE.md UI rule 11). `primary`/`secondary`/`danger` slots hold the actual
 * `AppButton`s — this block only owns the bar's layout, sticky positioning and the dirty message.
 */
import { AlertCircle } from '@lucide/vue';

withDefaults(defineProps<{ dirty?: boolean }>(), { dirty: false });
</script>

<template>
  <div class="no-print sticky bottom-0 z-10 -mx-4 flex flex-wrap items-center justify-between gap-3 border-t border-border bg-surface/95 px-4 py-3 backdrop-blur supports-[backdrop-filter]:bg-surface/80 sm:-mx-6 sm:px-6">
    <p v-if="dirty" class="flex items-center gap-1.5 text-xs text-warning">
      <AlertCircle class="size-3.5 shrink-0" />
      تغييرات غير محفوظة
    </p>
    <div v-else />
    <div class="flex flex-wrap items-center gap-2">
      <slot name="danger" />
      <slot name="secondary" />
      <slot name="primary" />
    </div>
  </div>
</template>
