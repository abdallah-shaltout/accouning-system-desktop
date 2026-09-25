<script setup lang="ts">
import { onMounted } from 'vue';
import { ConfigProvider } from 'reka-ui';
import CommandPalette from '@/modules/core/components/CommandPalette.vue';
import ToastContainer from '@/modules/core/components/ToastContainer.vue';
import ConfirmDialog from '@/modules/core/components/ui/ConfirmDialog.vue';
import { initPrintResultListener } from '@/modules/core/services/printService';
import { isClosingWithBackup } from '@/modules/settings/services/backupService';

// Phase 14 (docs/v2/12-documents-pdf-excel.md §5): one-time listener for the
// async native-print result event (toast + reprint + PDF fallback on
// failure). No-op outside Tauri.
onMounted(() => void initPrintResultListener());
</script>

<template>
  <!-- Phase C (docs/v2/16-equal-rebrand-and-ui-kit.md): mirrors menus, submenus, sliders and
       arrow-key navigation in every reka-ui-based shadcn component for this Arabic-only, RTL-only app. -->
  <ConfigProvider dir="rtl" locale="ar">
    <RouterView />
    <ToastContainer />
    <ConfirmDialog />
    <CommandPalette />
    <div v-if="isClosingWithBackup" class="closing-overlay" role="status" aria-live="polite">
      <div class="closing-overlay__box">
        <span class="closing-overlay__spinner" aria-hidden="true" />
        <span>جارٍ حفظ نسخة احتياطية قبل الإغلاق…</span>
      </div>
    </div>
  </ConfigProvider>
</template>

<style scoped>
.closing-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, black 45%, transparent);
}

.closing-overlay__box {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem 1.5rem;
  border-radius: 0.75rem;
  background: var(--color-background, #fff);
  color: var(--color-text-primary, #111);
  font-size: 0.95rem;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
}

.closing-overlay__spinner {
  width: 1.1rem;
  height: 1.1rem;
  border-radius: 50%;
  border: 2px solid color-mix(in srgb, currentColor 25%, transparent);
  border-top-color: currentColor;
  animation: closing-overlay-spin 0.8s linear infinite;
}

@keyframes closing-overlay-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
