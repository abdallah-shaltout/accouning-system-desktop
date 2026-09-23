<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { GripVertical, Image as ImageIcon, Star, Trash, Upload } from '@lucide/vue';
import { deleteAttachment, getAttachment, listAttachments, putAttachment, type AttachmentMeta } from '@/mocks/attachments';
import { processFile } from '@/modules/core/helpers/attachments';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';

/**
 * v2 §1 "أساسي" / §7 "Product images gallery": multiple images via the same AttachmentField blob
 * store, drag-to-reorder, first image = POS thumbnail. `ownerRef` is `product:<id>` — for a
 * not-yet-saved product the caller passes a temporary ref and the form re-attaches on save (see
 * ProductFormPage). `modelValue` is the ordered list of attachment ids, kept on `product.imageIds`.
 */
const props = defineProps<{ ownerRef: string; readonly?: boolean }>();
const imageIds = defineModel<string[]>({ default: () => [] });

const toast = useToast();
const confirm = useConfirm();
const auth = useAuthStore();

const items = ref<AttachmentMeta[]>([]);
const thumbUrls = ref<Record<string, string>>({});
const loading = ref(true);
const uploading = ref(false);
const dragOver = ref(false);
const fileInput = ref<HTMLInputElement>();
const dragIndex = ref<number | null>(null);

const ordered = computed(() => {
  const byId = new Map(items.value.map((i) => [i.id, i]));
  const inOrder = imageIds.value.map((id) => byId.get(id)).filter((x): x is AttachmentMeta => !!x);
  // Any attachment not yet in `imageIds` (just uploaded) tacks on at the end.
  const extra = items.value.filter((i) => !imageIds.value.includes(i.id));
  return [...inOrder, ...extra];
});

async function refresh() {
  loading.value = true;
  try {
    items.value = await listAttachments(props.ownerRef);
    // Reconcile order: drop stale ids, append new ones.
    const ids = items.value.map((i) => i.id);
    const kept = imageIds.value.filter((id) => ids.includes(id));
    const added = ids.filter((id) => !kept.includes(id));
    imageIds.value = [...kept, ...added];
    for (const item of items.value) void loadThumb(item);
  } finally {
    loading.value = false;
  }
}
onMounted(refresh);

async function loadThumb(meta: AttachmentMeta) {
  if (thumbUrls.value[meta.id]) return;
  const record = await getAttachment(meta.id);
  if (record?.thumbnail) thumbUrls.value = { ...thumbUrls.value, [meta.id]: URL.createObjectURL(record.thumbnail) };
  else if (record?.blob) thumbUrls.value = { ...thumbUrls.value, [meta.id]: URL.createObjectURL(record.blob) };
}

onBeforeUnmount(() => {
  for (const url of Object.values(thumbUrls.value)) URL.revokeObjectURL(url);
});

async function addFiles(files: FileList | File[]) {
  if (props.readonly) return;
  const list = Array.from(files).filter((f) => f.type.startsWith('image/'));
  if (!list.length) {
    if (files.length) toast.warning('اختر ملفات صور فقط');
    return;
  }
  uploading.value = true;
  try {
    for (const file of list) {
      try {
        const record = await processFile(file, props.ownerRef, auth.user?.id);
        await putAttachment(record);
        imageIds.value = [...imageIds.value, record.id];
      } catch (err) {
        toast.error(err, 'تعذر رفع الصورة');
      }
    }
    await refresh();
  } finally {
    uploading.value = false;
  }
}

function onInputChange(e: Event) {
  const input = e.target as HTMLInputElement;
  if (input.files) void addFiles(input.files);
  input.value = '';
}

function onDrop(e: DragEvent) {
  dragOver.value = false;
  if (e.dataTransfer?.files.length) void addFiles(e.dataTransfer.files);
}

async function remove(meta: AttachmentMeta) {
  if (props.readonly) return;
  const ok = await confirm({ title: 'حذف هذه الصورة؟', confirmText: 'حذف', danger: true });
  if (!ok) return;
  await deleteAttachment(meta.id);
  items.value = items.value.filter((i) => i.id !== meta.id);
  imageIds.value = imageIds.value.filter((id) => id !== meta.id);
  toast.success('تم حذف الصورة');
}

function makeThumbnail(id: string) {
  imageIds.value = [id, ...imageIds.value.filter((x) => x !== id)];
}

// --- drag-to-reorder ---
function onDragStart(index: number) {
  dragIndex.value = index;
}
function onDragOverItem(e: DragEvent) {
  e.preventDefault();
}
function onDropItem(index: number) {
  if (dragIndex.value === null || dragIndex.value === index) return;
  const next = [...ordered.value.map((i) => i.id)];
  const [moved] = next.splice(dragIndex.value, 1);
  next.splice(index, 0, moved);
  imageIds.value = next;
  dragIndex.value = null;
}
</script>

<template>
  <div>
    <div
      class="rounded-xl border border-dashed p-4 text-center transition-colors"
      :class="dragOver ? 'border-primary bg-primary/5' : 'border-border'"
      @dragover.prevent="dragOver = true"
      @dragleave.prevent="dragOver = false"
      @drop.prevent="onDrop"
    >
      <Upload class="mx-auto mb-2 size-5 text-text-secondary" />
      <p class="text-body">
        اسحب الصور هنا أو
        <button v-if="!readonly" type="button" class="text-primary hover:underline" @click="fileInput?.click()">اختر صوراً</button>
      </p>
      <p class="mt-1 text-xs text-text-secondary">الصورة الأولى تُستخدم كصورة مصغّرة في نقطة البيع — اسحب لإعادة الترتيب.</p>
      <input ref="fileInput" type="file" multiple accept="image/*" class="hidden" :disabled="readonly" @change="onInputChange" />
    </div>

    <p v-if="uploading" class="mt-2 text-xs text-text-secondary">جارٍ رفع الصور…</p>

    <div v-if="!loading && ordered.length" class="mt-3 grid grid-cols-3 gap-2.5 sm:grid-cols-4 md:grid-cols-5">
      <div
        v-for="(item, index) in ordered"
        :key="item.id"
        class="group relative flex flex-col overflow-hidden rounded-lg border bg-surface"
        :class="index === 0 ? 'border-primary' : 'border-border'"
        :draggable="!readonly"
        @dragstart="onDragStart(index)"
        @dragover="onDragOverItem"
        @drop="onDropItem(index)"
      >
        <div class="relative flex aspect-square items-center justify-center overflow-hidden bg-background">
          <img v-if="thumbUrls[item.id]" :src="thumbUrls[item.id]" :alt="item.name" class="h-full w-full object-cover" />
          <ImageIcon v-else class="size-8 text-text-secondary" :stroke-width="1.5" />
          <span v-if="index === 0" class="absolute top-1 start-1 inline-flex items-center gap-1 rounded-full bg-primary px-1.5 py-0.5 text-tiny text-on-primary">
            <Star class="size-2.5 fill-current" /> الرئيسية
          </span>
          <GripVertical v-if="!readonly" class="absolute top-1 end-1 size-3.5 text-white/80 opacity-0 drop-shadow group-hover:opacity-100" />
        </div>
        <div v-if="!readonly" class="flex items-center justify-between gap-1 px-1.5 py-1">
          <button
            v-if="index !== 0"
            type="button"
            class="rounded px-1 py-0.5 text-tiny text-text-secondary hover:bg-surface-hover hover:text-text-primary"
            @click="makeThumbnail(item.id)"
          >
            تعيين كرئيسية
          </button>
          <span v-else class="text-tiny text-text-secondary">&nbsp;</span>
          <button type="button" aria-label="حذف" class="shrink-0 rounded p-1 text-text-secondary hover:bg-danger/10 hover:text-danger" @click="remove(item)">
            <Trash class="size-3.5" />
          </button>
        </div>
      </div>
    </div>
    <p v-else-if="!loading" class="mt-3 text-xs text-text-secondary">لا توجد صور بعد</p>
  </div>
</template>
