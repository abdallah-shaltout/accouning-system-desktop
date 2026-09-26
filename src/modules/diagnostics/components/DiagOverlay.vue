<script setup lang="ts">
/**
 * `Ctrl+Shift+D` overlay (18.B5) — the last 50 log entries across every channel, docked to the
 * bottom of the screen, for a quick glance without leaving the current page. Dev builds only
 * (mounted from `App.vue` behind `import.meta.env.DEV`, same gate as `/dev/ui`/`/dev/diagnostics`).
 * `no-print` per CLAUDE.md rule 22 — app chrome never prints.
 */
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { X } from '@lucide/vue';
import { formatDateTime } from '@/modules/core/helpers/format';
import { readLogs } from '../services/diagnosticsService';
import type { LogChannel, LogEntry } from '../types';

const open = ref(false);
const entries = ref<LogEntry[]>([]);

const CHANNELS: LogChannel[] = ['error', 'perf', 'debug', 'audit', 'accounting'];

async function refresh(): Promise<void> {
  const today = new Date().toISOString().slice(0, 10);
  const all = (await Promise.all(CHANNELS.map((c) => readLogs(c, '2000-01-01', today)))).flat();
  entries.value = all.sort((a, b) => b.ts.localeCompare(a.ts)).slice(0, 50);
}

function onKeydown(e: KeyboardEvent): void {
  if (e.ctrlKey && e.shiftKey && e.code === 'KeyD') {
    e.preventDefault();
    open.value = !open.value;
    if (open.value) void refresh();
  }
}

onMounted(() => window.addEventListener('keydown', onKeydown));
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));
</script>

<template>
  <div v-if="open" class="no-print diag-overlay" role="log" aria-label="آخر 50 إدخال تشخيص">
    <div class="diag-overlay__header">
      <span>التشخيص — آخر 50 إدخال (Ctrl+Shift+D للإغلاق)</span>
      <button type="button" class="diag-overlay__close" @click="open = false"><X class="size-3.5" /></button>
    </div>
    <div class="diag-overlay__body">
      <div v-for="(e, i) in entries" :key="i" class="diag-overlay__row">
        <span class="num diag-overlay__ts">{{ formatDateTime(e.ts) }}</span>
        <span class="diag-overlay__channel" :data-level="e.level">{{ e.channel }}</span>
        <span class="diag-overlay__source">{{ e.source }}</span>
        <span class="diag-overlay__msg">{{ e.msg }}</span>
      </div>
      <p v-if="!entries.length" class="diag-overlay__empty">لا توجد إدخالات بعد.</p>
    </div>
  </div>
</template>

<style scoped>
.diag-overlay {
  position: fixed;
  inset-inline: 0;
  bottom: 0;
  z-index: 9998;
  max-height: 40vh;
  display: flex;
  flex-direction: column;
  background: color-mix(in srgb, black 88%, transparent);
  color: #e5e7eb;
  font-family: ui-monospace, monospace;
  font-size: 11px;
  border-top: 1px solid color-mix(in srgb, white 20%, transparent);
}

.diag-overlay__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.375rem 0.75rem;
  border-bottom: 1px solid color-mix(in srgb, white 15%, transparent);
  font-family: inherit;
}

.diag-overlay__close {
  color: inherit;
  opacity: 0.7;
}
.diag-overlay__close:hover {
  opacity: 1;
}

.diag-overlay__body {
  overflow-y: auto;
  padding: 0.25rem 0;
}

.diag-overlay__row {
  display: flex;
  gap: 0.5rem;
  padding: 0.125rem 0.75rem;
  white-space: nowrap;
  overflow: hidden;
}

.diag-overlay__ts {
  opacity: 0.6;
  flex-shrink: 0;
}

.diag-overlay__channel {
  flex-shrink: 0;
  opacity: 0.85;
}
.diag-overlay__channel[data-level='error'] {
  color: #f87171;
}
.diag-overlay__channel[data-level='warn'] {
  color: #fbbf24;
}

.diag-overlay__source {
  flex-shrink: 0;
  color: #60a5fa;
}

.diag-overlay__msg {
  overflow: hidden;
  text-overflow: ellipsis;
}

.diag-overlay__empty {
  padding: 0.5rem 0.75rem;
  opacity: 0.6;
}
</style>
