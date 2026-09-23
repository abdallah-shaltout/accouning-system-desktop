<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { ExternalLink, RotateCw, X, ZoomIn, ZoomOut } from '@lucide/vue';
import type { AttachmentMeta } from '@/mocks/attachments';
import { formatFileSize } from '../../helpers/attachments';
import AppButton from './AppButton.vue';

/**
 * Viewer for a single attachment: image lightbox (zoom/rotate), inline blob-iframe for PDFs, and
 * an "open externally" fallback for everything else (Office files, unknown types).
 */
const props = defineProps<{ meta: AttachmentMeta | null; url: string | null }>();
const emit = defineEmits<{ close: [] }>();

const zoom = ref(1);
const rotation = ref(0);

watch(
  () => props.meta,
  () => {
    zoom.value = 1;
    rotation.value = 0;
  },
);

const open = computed(() => !!props.meta && !!props.url);

function openExternally() {
  if (props.url) window.open(props.url, '_blank', 'noopener');
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close');
  else if (e.key === '+' || e.key === '=') zoom.value = Math.min(zoom.value + 0.25, 4);
  else if (e.key === '-') zoom.value = Math.max(zoom.value - 0.25, 0.25);
}

watch(open, (isOpen) => {
  if (isOpen) window.addEventListener('keydown', onKeydown);
  else window.removeEventListener('keydown', onKeydown);
});
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));
</script>

<template>
  <Teleport to="body">
    <div v-if="open && meta && url" dir="rtl" class="fixed inset-0 z-70 flex flex-col bg-black/85" @mousedown.self="emit('close')">
      <header class="no-print flex shrink-0 items-center justify-between gap-3 px-4 py-3 text-white">
        <div class="min-w-0">
          <p class="truncate text-body font-medium">{{ meta.name }}</p>
          <p class="text-xs text-white/60">{{ formatFileSize(meta.size) }}</p>
        </div>
        <div class="flex items-center gap-1.5">
          <template v-if="meta.kind === 'image'">
            <button type="button" class="rounded-md p-1.5 hover:bg-white/10" aria-label="تصغير" @click="zoom = Math.max(zoom - 0.25, 0.25)">
              <ZoomOut class="size-4" />
            </button>
            <button type="button" class="rounded-md p-1.5 hover:bg-white/10" aria-label="تكبير" @click="zoom = Math.min(zoom + 0.25, 4)">
              <ZoomIn class="size-4" />
            </button>
            <button type="button" class="rounded-md p-1.5 hover:bg-white/10" aria-label="تدوير" @click="rotation = (rotation + 90) % 360">
              <RotateCw class="size-4" />
            </button>
          </template>
          <a :href="url" target="_blank" rel="noopener" class="rounded-md p-1.5 hover:bg-white/10" aria-label="فتح خارجياً" title="فتح خارجياً">
            <ExternalLink class="size-4" />
          </a>
          <button type="button" class="rounded-md p-1.5 hover:bg-white/10" aria-label="إغلاق" @click="emit('close')">
            <X class="size-4" />
          </button>
        </div>
      </header>

      <div class="flex flex-1 items-center justify-center overflow-auto p-4">
        <img
          v-if="meta.kind === 'image'"
          :src="url"
          :alt="meta.name"
          class="max-h-full max-w-full select-none transition-transform"
          :style="{ transform: `scale(${zoom}) rotate(${rotation}deg)` }"
        />
        <iframe v-else-if="meta.kind === 'pdf'" :src="url" :title="meta.name" class="h-full w-full max-w-4xl rounded-lg bg-white" />
        <div v-else class="flex flex-col items-center gap-3 rounded-xl border border-white/15 bg-white/5 px-8 py-10 text-center text-white">
          <p class="text-body">لا يمكن معاينة هذا النوع من الملفات داخل التطبيق</p>
          <AppButton :icon="ExternalLink" variant="primary" @click="openExternally">فتح خارجياً</AppButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>
