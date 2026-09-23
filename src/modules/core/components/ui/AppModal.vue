<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { X } from '@lucide/vue';

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
const panel = ref<HTMLElement>();

const widths = { sm: 'max-w-sm', md: 'max-w-lg', lg: 'max-w-2xl', xl: 'max-w-4xl' };

function close() {
  if (!props.persistent) open.value = false;
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && open.value) {
    e.stopPropagation();
    close();
  }
}

watch(open, async (isOpen) => {
  if (isOpen) {
    window.addEventListener('keydown', onKeydown);
    await nextTick();
    const first = panel.value?.querySelector<HTMLElement>('[autofocus], input:not([type=hidden]), select, textarea, button:not([data-close])');
    first?.focus();
  } else {
    window.removeEventListener('keydown', onKeydown);
  }
}, { immediate: true });

onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-150 ease-out"
      enter-from-class="opacity-0"
      leave-active-class="transition duration-100 ease-in"
      leave-to-class="opacity-0"
    >
      <div v-if="open" dir="rtl" class="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-black/45 p-4 pt-[8vh]" @mousedown.self="close">
        <div
          ref="panel"
          role="dialog"
          aria-modal="true"
          :aria-label="title"
          class="w-full rounded-xl border border-border bg-background shadow-2xl"
          :class="widths[size]"
        >
          <header v-if="title" class="flex items-start justify-between gap-4 border-b border-border px-5 py-4">
            <div>
              <h2 class="text-[15px] font-semibold">{{ title }}</h2>
              <p v-if="description" class="mt-0.5 text-xs text-text-secondary">{{ description }}</p>
            </div>
            <button
              type="button"
              data-close
              aria-label="إغلاق"
              class="rounded-md p-1 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
              @click="close"
            >
              <X class="size-4" />
            </button>
          </header>
          <div class="px-5 py-4">
            <slot />
          </div>
          <footer v-if="$slots.footer" class="flex items-center justify-end gap-2 border-t border-border px-5 py-3">
            <slot name="footer" />
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
