<script setup lang="ts">
import { onMounted } from 'vue';
import CommandPalette from '@/modules/core/components/CommandPalette.vue';
import ToastContainer from '@/modules/core/components/ToastContainer.vue';
import ConfirmDialog from '@/modules/core/components/ui/ConfirmDialog.vue';
import { initPrintResultListener } from '@/modules/core/services/printService';

// Phase 14 (docs/v2/12-documents-pdf-excel.md §5): one-time listener for the
// async native-print result event (toast + reprint + PDF fallback on
// failure). No-op outside Tauri.
onMounted(() => void initPrintResultListener());
</script>

<template>
  <RouterView />
  <ToastContainer />
  <ConfirmDialog />
  <CommandPalette />
</template>
