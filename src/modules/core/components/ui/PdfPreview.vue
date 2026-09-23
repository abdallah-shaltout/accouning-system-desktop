<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ChevronLeft, ChevronRight, ZoomIn, ZoomOut } from '@lucide/vue';
import AppButton from './AppButton.vue';
import SkeletonBlock from './SkeletonBlock.vue';

/**
 * Renders the SVG pages returned by `render_preview` — a page list (thumbnails
 * strip when >1 page), zoom controls and page navigation, per
 * docs/v2/12-documents-pdf-excel.md §3 "Live preview: SVG pages ... Uses
 * sample data or a real document". The caller owns re-rendering (debounced
 * elsewhere); this component only displays whatever `pages` it's given.
 */
const props = withDefaults(defineProps<{ pages: string[]; loading?: boolean; emptyMessage?: string }>(), {
  loading: false,
  emptyMessage: 'لا توجد معاينة بعد',
});

const currentPage = ref(0);
const zoom = ref(1);

watch(
  () => props.pages.length,
  (len) => {
    if (currentPage.value >= len) currentPage.value = Math.max(0, len - 1);
  },
);

const activeSvg = computed(() => props.pages[currentPage.value] ?? null);

function zoomIn() {
  zoom.value = Math.min(2.5, Math.round((zoom.value + 0.15) * 100) / 100);
}
function zoomOut() {
  zoom.value = Math.max(0.4, Math.round((zoom.value - 0.15) * 100) / 100);
}
function resetZoom() {
  zoom.value = 1;
}
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="flex items-center justify-between gap-2 border-b border-border bg-surface px-3 py-2">
      <div class="flex items-center gap-1">
        <AppButton size="sm" variant="ghost" :icon="ChevronRight" :disabled="currentPage <= 0" aria-label="الصفحة السابقة" @click="currentPage--" />
        <span class="num min-w-16 text-center text-xs text-text-secondary">
          {{ pages.length ? `${currentPage + 1} / ${pages.length}` : '—' }}
        </span>
        <AppButton size="sm" variant="ghost" :icon="ChevronLeft" :disabled="currentPage >= pages.length - 1" aria-label="الصفحة التالية" @click="currentPage++" />
      </div>
      <div class="flex items-center gap-1">
        <AppButton size="sm" variant="ghost" :icon="ZoomOut" aria-label="تصغير" @click="zoomOut" />
        <button type="button" class="num w-12 text-center text-xs text-text-secondary hover:text-text-primary" @click="resetZoom">{{ Math.round(zoom * 100) }}%</button>
        <AppButton size="sm" variant="ghost" :icon="ZoomIn" aria-label="تكبير" @click="zoomIn" />
      </div>
    </div>

    <div class="flex flex-1 items-start justify-center overflow-auto bg-surface-hover p-6">
      <div v-if="loading" class="w-[210mm] max-w-full rounded-md bg-white p-8 shadow-sm">
        <SkeletonBlock :lines="10" />
      </div>
      <div v-else-if="!activeSvg" class="flex h-64 items-center justify-center text-body text-text-secondary">
        {{ emptyMessage }}
      </div>
      <div
        v-else
        class="origin-top bg-white shadow-md transition-transform"
        :style="{ transform: `scale(${zoom})` }"
        v-html="activeSvg"
      />
    </div>

    <div v-if="pages.length > 1" class="flex gap-2 overflow-x-auto border-t border-border bg-surface px-3 py-2">
      <button
        v-for="(p, i) in pages"
        :key="i"
        type="button"
        class="shrink-0 overflow-hidden rounded border-2 bg-white transition-colors"
        :class="i === currentPage ? 'border-primary' : 'border-border hover:border-text-secondary'"
        :style="{ width: '48px', height: '68px' }"
        @click="currentPage = i"
        v-html="p"
      />
    </div>
  </div>
</template>
