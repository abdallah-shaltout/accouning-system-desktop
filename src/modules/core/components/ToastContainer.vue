<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from '@lucide/vue';
import { useNotificationStore } from '../controllers/useNotificationStore';

const store = useNotificationStore();
const { notifications } = storeToRefs(store);

const icons = { success: CircleCheck, error: CircleAlert, warning: TriangleAlert, info: Info };
const tones = { success: 'text-success', error: 'text-danger', warning: 'text-warning', info: 'text-primary' };
</script>

<template>
  <div class="no-print pointer-events-none fixed bottom-4 left-4 z-[70] flex w-[340px] max-w-[calc(100vw-2rem)] flex-col gap-2" aria-live="polite">
    <TransitionGroup
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0 -translate-x-4"
      leave-active-class="transition duration-150 ease-in absolute"
      leave-to-class="opacity-0"
      move-class="transition duration-200"
    >
      <div
        v-for="toast in notifications"
        :key="toast.id"
        role="status"
        class="pointer-events-auto flex items-start gap-3 rounded-lg border border-border bg-background p-3 shadow-lg"
      >
        <component :is="icons[toast.type]" class="mt-0.5 size-4 shrink-0" :class="tones[toast.type]" />
        <div class="min-w-0 flex-1">
          <p class="text-body font-medium">{{ toast.title }}</p>
          <p v-if="toast.message" class="mt-0.5 text-xs leading-5 text-text-secondary">{{ toast.message }}</p>
        </div>
        <button
          type="button"
          aria-label="إغلاق"
          class="rounded p-0.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
          @click="store.removeNotification(toast.id)"
        >
          <X class="size-3.5" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>
