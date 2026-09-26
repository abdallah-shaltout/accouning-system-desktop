<script setup lang="ts">
/**
 * v2 doc 17 Phase F-5: shared logo/stamp/signature upload control used by
 * `GeneralSettingsPage`. Keeps the base64 data-URL pattern (not `AttachmentField`'s async
 * IndexedDB blob store) — `pdfService.ts`'s `companyBlock()` reads these fields synchronously by
 * value when building a Typst document payload, so the image has to be an inline string, not a
 * blob reference (see `GeneralSettingsPage.vue`'s header comment).
 *
 * Moving the hidden `<input type="file">` into this component (rather than a page) keeps
 * `modules/*\/pages/*.vue` free of bare `<input>` elements per CLAUDE.md UI rule 7 / doc 17 F2 rule 3.
 */
import { ref } from 'vue';
import { ImagePlus, Trash, X, ZoomIn } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import { useToast } from '@/modules/core/controllers/useToast';

const props = withDefaults(
  defineProps<{
    modelValue?: string;
    label: string;
    changeLabel: string;
    alt: string;
    disabled?: boolean;
    size?: 'sm' | 'md';
    maxKb?: number;
  }>(),
  { size: 'md', maxKb: 600 },
);

const emit = defineEmits<{ 'update:modelValue': [value: string | undefined] }>();

const toast = useToast();
const fileInput = ref<HTMLInputElement>();
const dragOver = ref(false);
const previewOpen = ref(false);

function handleFile(file: File | undefined) {
  if (!file) return;
  if (!file.type.startsWith('image/')) return toast.warning('اختر ملف صورة');
  if (file.size > props.maxKb * 1024) return toast.warning(`حجم ${props.label} كبير`, `الحد الأقصى ${props.maxKb} كيلوبايت`);
  const reader = new FileReader();
  reader.onload = () => emit('update:modelValue', String(reader.result));
  reader.readAsDataURL(file);
}

function onChange(e: Event) {
  const input = e.target as HTMLInputElement;
  handleFile(input.files?.[0]);
  input.value = '';
}

function onDrop(e: DragEvent) {
  dragOver.value = false;
  if (props.disabled) return;
  handleFile(e.dataTransfer?.files[0]);
}

function openPicker() {
  if (!props.disabled) fileInput.value?.click();
}
</script>

<template>
  <div class="flex items-center gap-4">
    <button
      type="button"
      :disabled="disabled"
      class="group relative flex shrink-0 items-center justify-center overflow-hidden rounded-lg border border-dashed border-border bg-background transition-[border-color,background-color] duration-150 ease-out disabled:cursor-default"
      :class="[size === 'sm' ? 'size-16' : 'size-20', dragOver ? 'border-primary bg-primary/5' : 'hover:border-primary/60']"
      @click="modelValue ? (previewOpen = true) : openPicker()"
      @dragover.prevent="!disabled && (dragOver = true)"
      @dragleave.prevent="dragOver = false"
      @drop.prevent="onDrop"
    >
      <img v-if="modelValue" :src="modelValue" :alt="alt" class="size-full object-contain" />
      <ImagePlus
        v-else
        class="text-text-secondary transition-transform duration-150 ease-out"
        :class="[size === 'sm' ? 'size-5' : 'size-6', dragOver && 'scale-110 text-primary']"
      />
      <div v-if="modelValue" class="absolute inset-0 flex items-center justify-center bg-black/0 opacity-0 transition-[opacity,background-color] duration-150 ease-out group-hover:bg-black/40 group-hover:opacity-100">
        <ZoomIn class="size-5 text-white" />
      </div>
    </button>
    <div class="space-y-2">
      <input ref="fileInput" type="file" accept="image/*" class="hidden" :disabled="disabled" @change="onChange" />
      <AppButton size="sm" :icon="ImagePlus" :disabled="disabled" @click="openPicker">{{ modelValue ? changeLabel : label }}</AppButton>
      <AppButton v-if="modelValue" size="sm" variant="ghost" :icon="Trash" :disabled="disabled" @click="emit('update:modelValue', undefined)">إزالة</AppButton>
      <p class="text-caption text-text-secondary">أو اسحب صورة وأفلتها هنا</p>
    </div>

    <Teleport to="body">
      <div v-if="previewOpen && modelValue" dir="rtl" class="fixed inset-0 z-70 flex items-center justify-center bg-black/85 p-6" @mousedown.self="previewOpen = false">
        <button type="button" class="absolute top-4 end-4 rounded-md p-1.5 text-white hover:bg-white/10" aria-label="إغلاق" @click="previewOpen = false">
          <X class="size-5" />
        </button>
        <img :src="modelValue" :alt="alt" class="max-h-full max-w-full select-none object-contain" />
      </div>
    </Teleport>
  </div>
</template>
