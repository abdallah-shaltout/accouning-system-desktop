<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { Bug } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import { formatDateTime } from '@/modules/core/helpers/format';
import { KNOWN_NAMESPACES, useDebugNamespaces } from '../../controllers/useDebugNamespaces';
import { loadChannel } from '../../services/diagnosticsReadService';
import type { LogEntry } from '../../types';

/** Namespace toggles + a polling "live" tail of the `debug` channel (18.B5). Off by default, per
 * device — `useDebugNamespaces` is the single reader/writer of `localStorage['equal.debug']`. */
const { namespaces, isEnabled, toggle } = useDebugNamespaces();
const entries = ref<LogEntry[]>([]);
const namespaceFilter = ref('');

async function refresh() {
  const all = await loadChannel('debug', 1);
  entries.value = namespaceFilter.value ? all.filter((e) => e.source.includes(namespaceFilter.value)) : all;
  entries.value = entries.value.slice(-200).reverse();
}

let timer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  void refresh();
  timer = setInterval(refresh, 3000);
});
onBeforeUnmount(() => clearInterval(timer));
</script>

<template>
  <div class="space-y-4">
    <AppCard title="تشغيل التتبع" padding="sm">
      <div class="flex flex-wrap gap-2">
        <AppButton
          v-for="ns in KNOWN_NAMESPACES"
          :key="ns"
          size="sm"
          :variant="isEnabled(ns) ? 'primary' : 'secondary'"
          @click="toggle(ns)"
        >
          {{ ns }}
        </AppButton>
      </div>
      <p class="mt-2 text-xs text-text-secondary">
        فعّالة الآن: <span class="num">{{ namespaces.size ? [...namespaces].join('، ') : 'لا شيء' }}</span> —
        يمكن أيضاً كتابتها في localStorage['equal.debug'] مثل posting,pos.*
      </p>
    </AppCard>

    <SearchInput v-model="namespaceFilter" placeholder="تصفية حسب النطاق…" @update:model-value="refresh" />

    <AppCard padding="none">
      <div class="max-h-112 overflow-y-auto font-mono text-xs">
        <div v-for="(e, i) in entries" :key="i" class="border-b border-border px-3 py-1.5 last:border-0">
          <span class="num text-text-secondary">{{ formatDateTime(e.ts) }}</span>
          <span class="mx-2 text-primary">{{ e.source }}</span>
          <span>{{ e.msg }}</span>
        </div>
        <p v-if="!entries.length" class="flex items-center gap-2 p-4 text-text-secondary"><Bug class="size-4" /> لا توجد إدخالات تتبع — فعّل نطاقاً أعلاه وتصفّح التطبيق.</p>
      </div>
    </AppCard>
  </div>
</template>
