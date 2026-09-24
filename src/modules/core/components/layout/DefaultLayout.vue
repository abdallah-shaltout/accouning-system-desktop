<script setup lang="ts">
import { ref, watch } from 'vue';
import { sidebarCollapsedDefault } from '../../controllers/useAppearance';
import AppSidebar from './AppSidebar.vue';
import AppTopbar from './AppTopbar.vue';
import ErrorBoundary from '../ErrorBoundary.vue';
import KeyboardShortcutsSheet from './KeyboardShortcutsSheet.vue';

const COLLAPSE_KEY = 'app_sidebar_collapsed';
// The "القائمة الجانبية: مطوية افتراضياً" appearance setting seeds this session's initial value;
// once the user toggles it by hand, that per-session choice (below) wins until they clear it.
const collapsed = ref(sidebarCollapsedDefault.value);
try {
  const saved = localStorage.getItem(COLLAPSE_KEY);
  if (saved !== null) collapsed.value = saved === '1';
} catch {
  /* ignore */
}
watch(collapsed, (v) => {
  try {
    localStorage.setItem(COLLAPSE_KEY, v ? '1' : '0');
  } catch {
    /* ignore */
  }
});
</script>

<template>
  <div class="flex h-screen overflow-hidden bg-background">
    <AppSidebar v-model:collapsed="collapsed" />
    <div class="flex min-w-0 flex-1 flex-col">
      <AppTopbar />
      <main class="flex-1 overflow-y-auto">
        <div class="mx-auto w-full max-w-[1400px] px-6 py-6">
          <RouterView v-slot="{ Component, route }">
            <ErrorBoundary :key="route.path">
              <component :is="Component" />
            </ErrorBoundary>
          </RouterView>
        </div>
      </main>
    </div>
    <KeyboardShortcutsSheet />
  </div>
</template>
