<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { File as FileIcon, FileSpreadsheet, FileText, Image as ImageIcon, Paperclip, Trash, Upload } from '@lucide/vue';
import { deleteAttachment, getAttachment, listAttachments, putAttachment, type AttachmentKind, type AttachmentMeta } from '@/mocks/attachments';
import { ACCEPT_ATTR, formatFileSize, processFile } from '../../helpers/attachments';
import { useConfirm } from '../../controllers/useConfirm';
import { useToast } from '../../controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import AttachmentViewer from './AttachmentViewer.vue';

/**
 * Drop zone + file picker + paste-from-clipboard for attaching files to a record (`ownerRef`,
 * e.g. `customer:cus-3`). Images are resized/re-encoded client-side (see `helpers/attachments.ts`);
 * everything else is stored as-is in the IndexedDB blob store (`mocks/attachments.ts`).
 *
 * Audit rule (docs/v2/14-platform.md §5): on posted accounting documents, attachments can be
 * *added* freely but *removed* only by an admin, with the removal logged — pass `restrictRemove`
 * once a phase wires this onto a posted document; this demo usage leaves it off.
 */
const props = withDefaults(
  defineProps<{
    ownerRef: string;
    /** When true, only admins may remove an attachment (posted-document audit rule). */
    restrictRemove?: boolean;
    readonly?: boolean;
  }>(),
  { restrictRemove: false, readonly: false },
);

const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();

const items = ref<AttachmentMeta[]>([]);
const loading = ref(true);
const dragOver = ref(false);
const uploading = ref(false);
const fileInput = ref<HTMLInputElement>();

const viewerMeta = ref<AttachmentMeta | null>(null);
const viewerUrl = ref<string | null>(null);

const canRemove = computed(() => !props.restrictRemove || auth.role === 'admin');

const KIND_ICON: Record<AttachmentKind, typeof FileIcon> = {
  image: ImageIcon,
  pdf: FileText,
  office: FileSpreadsheet,
  other: FileIcon,
};

async function refresh() {
  loading.value = true;
  try {
    items.value = await listAttachments(props.ownerRef);
  } finally {
    loading.value = false;
  }
}
onMounted(refresh);

async function addFiles(files: FileList | File[]) {
  if (props.readonly) return;
  const list = Array.from(files);
  if (!list.length) return;
  uploading.value = true;
  try {
    for (const file of list) {
      try {
        const record = await processFile(file, props.ownerRef, auth.user?.id);
        await putAttachment(record);
      } catch (err) {
        toast.error(err, 'تعذر إرفاق الملف');
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

function onPaste(e: ClipboardEvent) {
  if (props.readonly) return;
  const files = Array.from(e.clipboardData?.items ?? [])
    .filter((i) => i.kind === 'file')
    .map((i) => i.getAsFile())
    .filter((f): f is File => !!f);
  if (files.length) void addFiles(files);
}

onMounted(() => document.addEventListener('paste', onPaste));
onBeforeUnmount(() => {
  document.removeEventListener('paste', onPaste);
  if (viewerUrl.value) URL.revokeObjectURL(viewerUrl.value);
});

async function openViewer(meta: AttachmentMeta) {
  const record = await getAttachment(meta.id);
  if (!record) return;
  if (viewerUrl.value) URL.revokeObjectURL(viewerUrl.value);
  viewerUrl.value = URL.createObjectURL(record.blob);
  viewerMeta.value = meta;
}

function closeViewer() {
  viewerMeta.value = null;
  if (viewerUrl.value) URL.revokeObjectURL(viewerUrl.value);
  viewerUrl.value = null;
}

async function remove(meta: AttachmentMeta) {
  if (!canRemove.value) return;
  const ok = await confirm({ title: `حذف "${meta.name}"؟`, confirmText: 'حذف', danger: true });
  if (!ok) return;
  await deleteAttachment(meta.id);
  items.value = items.value.filter((i) => i.id !== meta.id);
  toast.success('تم حذف المرفق');
}

const thumbUrls = ref<Record<string, string>>({});
async function loadThumb(meta: AttachmentMeta) {
  if (meta.kind !== 'image' || thumbUrls.value[meta.id]) return;
  const record = await getAttachment(meta.id);
  if (record?.thumbnail) thumbUrls.value = { ...thumbUrls.value, [meta.id]: URL.createObjectURL(record.thumbnail) };
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
        اسحب الملفات هنا أو
        <button v-if="!readonly" type="button" class="text-primary hover:underline" @click="fileInput?.click()">اختر ملفاً</button>
      </p>
      <p class="mt-1 text-xs text-text-secondary">صور، PDF، أو ملفات Office — حتى 10 ميجابايت. يمكنك أيضاً اللصق من الحافظة (Ctrl+V).</p>
      <input
        ref="fileInput"
        type="file"
        multiple
        class="hidden"
        :accept="ACCEPT_ATTR"
        :disabled="readonly"
        @change="onInputChange"
      />
    </div>

    <p v-if="uploading" class="mt-2 text-xs text-text-secondary">جارٍ رفع الملفات…</p>

    <div v-if="!loading && items.length" class="mt-3 grid grid-cols-2 gap-2.5 sm:grid-cols-3 md:grid-cols-4">
      <div
        v-for="item in items"
        :key="item.id"
        class="group relative flex cursor-pointer flex-col overflow-hidden rounded-lg border border-border bg-surface"
        @click="openViewer(item)"
        @mouseenter="loadThumb(item)"
      >
        <div class="flex aspect-square items-center justify-center overflow-hidden bg-background">
          <img v-if="item.kind === 'image' && thumbUrls[item.id]" :src="thumbUrls[item.id]" :alt="item.name" class="h-full w-full object-cover" />
          <component :is="KIND_ICON[item.kind]" v-else class="size-8 text-text-secondary" :stroke-width="1.5" />
        </div>
        <div class="flex items-center justify-between gap-1 px-2 py-1.5">
          <div class="min-w-0">
            <p class="truncate text-xs font-medium">{{ item.name }}</p>
            <p class="text-caption text-text-secondary">{{ formatFileSize(item.size) }}</p>
          </div>
          <button
            v-if="canRemove && !readonly"
            type="button"
            aria-label="حذف"
            class="shrink-0 rounded p-1 text-text-secondary opacity-0 hover:bg-danger/10 hover:text-danger group-hover:opacity-100"
            @click.stop="remove(item)"
          >
            <Trash class="size-3.5" />
          </button>
        </div>
      </div>
    </div>
    <p v-else-if="!loading" class="mt-3 flex items-center gap-1.5 text-xs text-text-secondary">
      <Paperclip class="size-3.5" /> لا توجد مرفقات بعد
    </p>

    <AttachmentViewer :meta="viewerMeta" :url="viewerUrl" @close="closeViewer" />
  </div>
</template>
