<script setup lang="ts">
/**
 * Rebuilt on shadcn's Dialog, i.e. reka-ui's DialogRoot/DialogContent (docs/v2/16-equal-rebrand-and-
 * ui-kit.md Phase C) — same props/slots/model as before. This is a real accessibility upgrade over
 * the hand-rolled version: focus trap, scroll lock and Escape handling now come from reka-ui instead
 * of the manual `nextTick()` autofocus + window keydown listener this used to do.
 *
 * `persistent` maps to reka-ui's own `DialogContent` escape/pointer-down-outside prevention, and the
 * visual chrome (header with an "X" close button, footer, sizes) is kept exactly as it was, on top
 * of `DialogContent`'s built-in overlay/portal/animation.
 */
import { X } from '@lucide/vue';
import { Dialog, DialogContent, DialogTitle, DialogDescription } from '@/modules/core/components/shadcn/dialog';

const props = withDefaults(
  defineProps<{
    title?: string;
    description?: string;
    size?: 'sm' | 'md' | 'lg' | 'xl';
    /** Prevent closing by overlay click / Escape (e.g. while saving). */
    persistent?: boolean;
  }>(),
  { size: 'md' },
);

const open = defineModel<boolean>('open', { default: false });

const widths = { sm: 'max-w-sm', md: 'max-w-lg', lg: 'max-w-2xl', xl: 'max-w-4xl' };

function onEscapeKeyDown(e: Event) {
  if (props.persistent) e.preventDefault();
}
function onPointerDownOutside(e: Event) {
  if (props.persistent) e.preventDefault();
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      :show-close-button="false"
      dir="rtl"
      class="grid-rows-[auto_1fr_auto] gap-0 rounded-xl border-border bg-background p-0 shadow-2xl"
      :class="widths[size]"
      @escape-key-down="onEscapeKeyDown"
      @pointer-down-outside="onPointerDownOutside"
    >
      <header v-if="title" class="flex items-start justify-between gap-4 border-b border-border px-5 py-4">
        <div>
          <DialogTitle class="text-lead font-semibold">{{ title }}</DialogTitle>
          <DialogDescription v-if="description" class="mt-0.5 text-xs text-text-secondary">{{ description }}</DialogDescription>
        </div>
        <button
          type="button"
          data-close
          aria-label="إغلاق"
          class="rounded-md p-1 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
          @click="open = false"
        >
          <X class="size-4" />
        </button>
      </header>
      <DialogTitle v-else class="sr-only">{{ ' ' }}</DialogTitle>
      <div class="px-5 py-4">
        <slot />
      </div>
      <footer v-if="$slots.footer" class="flex items-center justify-end gap-2 border-t border-border px-5 py-3">
        <slot name="footer" />
      </footer>
    </DialogContent>
  </Dialog>
</template>
