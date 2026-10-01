<script setup lang="ts">
import { ref } from "vue";
import { UploadCloudIcon } from "@lucide/vue";
import { cn } from "@/shared/helpers/utils";

/**
 * Drag-and-drop + click-to-browse file input, for receipt/screenshot uploads (05-ui-rules.md
 * "Portal on phones" — `capture` lets a phone open its camera directly for a receipt photo).
 */
interface Props {
  accept?: string;
  capture?: boolean | "user" | "environment";
  multiple?: boolean;
  disabled?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  accept: "image/*,application/pdf",
  multiple: false,
});

const emit = defineEmits<{ (e: "select", files: FileList): void }>();

const inputRef = ref<HTMLInputElement | null>(null);
const isDragging = ref(false);

function openBrowser(): void {
  if (props.disabled) return;
  inputRef.value?.click();
}

function onInputChange(event: Event): void {
  const files = (event.target as HTMLInputElement).files;
  if (files && files.length > 0) emit("select", files);
}

function onDrop(event: DragEvent): void {
  isDragging.value = false;
  if (props.disabled) return;
  const files = event.dataTransfer?.files;
  if (files && files.length > 0) emit("select", files);
}
</script>

<template>
  <div
    role="button"
    tabindex="0"
    :class="cn(
      'flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border-control px-4 py-8 text-center transition-colors',
      isDragging ? 'border-primary bg-primary/5' : 'hover:bg-surface-hover',
      disabled && 'pointer-events-none opacity-60',
    )"
    @click="openBrowser"
    @keydown.enter="openBrowser"
    @keydown.space.prevent="openBrowser"
    @dragover.prevent="isDragging = true"
    @dragleave.prevent="isDragging = false"
    @drop.prevent="onDrop"
  >
    <UploadCloudIcon class="size-6 text-text-secondary" />
    <p class="text-body text-text-secondary">
      <slot>اسحب ملفًا هنا أو اضغط للاختيار</slot>
    </p>
    <input
      ref="inputRef"
      type="file"
      class="sr-only"
      :accept="accept"
      :capture="capture"
      :multiple="multiple"
      :disabled="disabled"
      @change="onInputChange"
    />
  </div>
</template>
