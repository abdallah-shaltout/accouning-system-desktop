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
import { ImagePlus, Trash } from '@lucide/vue';
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

function onChange(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (!file) return;
  if (!file.type.startsWith('image/')) return toast.warning('اختر ملف صورة');
  if (file.size > props.maxKb * 1024) return toast.warning(`حجم ${props.label} كبير`, `الحد الأقصى ${props.maxKb} كيلوبايت`);
  const reader = new FileReader();
  reader.onload = () => emit('update:modelValue', String(reader.result));
  reader.readAsDataURL(file);
}
</script>

<template>
  <div class="flex items-center gap-4">
    <div
      class="flex shrink-0 items-center justify-center overflow-hidden rounded-lg border border-dashed border-border bg-background"
      :class="size === 'sm' ? 'size-16' : 'size-20'"
    >
      <img v-if="modelValue" :src="modelValue" :alt="alt" class="size-full object-contain" />
      <ImagePlus v-else :class="size === 'sm' ? 'size-5' : 'size-6'" class="text-text-secondary" />
    </div>
    <div class="space-y-2">
      <input ref="fileInput" type="file" accept="image/*" class="hidden" @change="onChange" />
      <AppButton size="sm" :icon="ImagePlus" :disabled="disabled" @click="fileInput?.click()">{{ modelValue ? changeLabel : label }}</AppButton>
      <AppButton v-if="modelValue" size="sm" variant="ghost" :icon="Trash" :disabled="disabled" @click="emit('update:modelValue', undefined)">إزالة</AppButton>
    </div>
  </div>
</template>
