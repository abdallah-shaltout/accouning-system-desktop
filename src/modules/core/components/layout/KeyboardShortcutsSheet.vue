<script setup lang="ts">
/**
 * Global keyboard-shortcuts sheet (docs/v2/14-platform.md §7 "F1 / ? opens the keyboard-shortcuts
 * sheet for the current page"). Mounted once in `DefaultLayout.vue`. POS already has its own
 * extensive F1 sheet (Phase 7) scoped to itself — this one is skipped on `/pos` so the two don't
 * both pop up, and shows whatever `ROUTE_SHORTCUTS` has for every other page, falling back to just
 * the global shortcuts (Ctrl+K, F1/?) when the current page has none of its own.
 */
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { Keyboard } from '@lucide/vue';
import AppModal from '../ui/AppModal.vue';
import { Kbd } from '@/modules/core/components/shadcn/kbd';
import { useHotkeys } from '../../controllers/useHotkeys';
import { keyboardShortcutsOpen as open, useKeyboardShortcutsSheet } from '../../controllers/useKeyboardShortcutsSheet';
import { GLOBAL_SHORTCUTS, ROUTE_SHORTCUTS } from '../../helpers/keyboardShortcuts';

const route = useRoute();
const { toggle } = useKeyboardShortcutsSheet();

const pageShortcuts = computed(() => (typeof route.name === 'string' ? (ROUTE_SHORTCUTS[route.name] ?? []) : []));

useHotkeys(
  {
    F1: toggle,
    '?': toggle,
  },
  { enabled: () => route.name !== 'pos' },
);
</script>

<template>
  <AppModal v-model:open="open" title="اختصارات لوحة المفاتيح" size="sm">
    <div class="space-y-4">
      <div v-if="pageShortcuts.length">
        <p class="mb-2 text-tiny font-medium text-text-secondary">لهذه الصفحة</p>
        <ul class="space-y-1">
          <li v-for="s in pageShortcuts" :key="s.keys" class="flex items-center justify-between gap-3 py-1 text-body">
            <span class="text-text-secondary">{{ s.label }}</span>
            <Kbd class="num">{{ s.keys }}</Kbd>
          </li>
        </ul>
      </div>
      <div v-else class="flex flex-col items-center gap-2 py-6 text-center text-text-secondary">
        <Keyboard class="size-6" :stroke-width="1.5" />
        <p class="text-body">لا توجد اختصارات خاصة بهذه الصفحة</p>
      </div>
      <div>
        <p class="mb-2 text-tiny font-medium text-text-secondary">عامة</p>
        <ul class="space-y-1">
          <li v-for="s in GLOBAL_SHORTCUTS" :key="s.keys" class="flex items-center justify-between gap-3 py-1 text-body">
            <span class="text-text-secondary">{{ s.label }}</span>
            <Kbd class="num">{{ s.keys }}</Kbd>
          </li>
        </ul>
      </div>
    </div>
  </AppModal>
</template>
