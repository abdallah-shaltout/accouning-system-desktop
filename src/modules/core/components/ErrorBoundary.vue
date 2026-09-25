<script setup lang="ts">
import { onErrorCaptured, ref } from 'vue';
import ErrorState from './ui/ErrorState.vue';
import { log } from '@/modules/diagnostics/services/logService';

/**
 * Catches render/lifecycle errors from a subtree (a whole page) and shows a recoverable error
 * state instead of a blank screen. `retry` remounts the subtree.
 */
const error = ref<string | null>(null);
const key = ref(0);

onErrorCaptured((err) => {
  console.error('[ErrorBoundary]', err);
  log.error('ui.ErrorBoundary', err instanceof Error ? err.message : String(err), err instanceof Error ? err : new Error(String(err)));
  error.value = err instanceof Error ? err.message : String(err);
  return false;
});

function retry() {
  error.value = null;
  key.value++;
}
</script>

<template>
  <ErrorState v-if="error" title="حدث خطأ أثناء عرض هذه الصفحة" :message="error" @retry="retry" />
  <div v-else :key="key" class="contents">
    <slot />
  </div>
</template>
