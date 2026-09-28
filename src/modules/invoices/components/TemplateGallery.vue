<script setup lang="ts">
import { computed } from 'vue';
import { Check } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import { IMAGE_TEMPLATE_WIDTH_PX, templatesFor, type TemplateId } from '../helpers/invoiceTemplates';
import type { PrintData } from '../services/invoiceService';
import InvoiceDocument from './InvoiceDocument.vue';

/**
 * Plan 22: the print page's template picker — a live, scaled-down render of the real invoice in every
 * template of the current layout, so the choice is made by looking, not by reading names.
 */
const props = defineProps<{ data: PrintData; layout: 'a4' | 'image'; defaultId: TemplateId; canSetDefault: boolean; savingDefault?: boolean }>();
const model = defineModel<TemplateId>({ required: true });
const emit = defineEmits<{ setDefault: [id: TemplateId] }>();

const templates = computed(() => templatesFor(props.layout));

// Thumbnail geometry: A4 is 210 mm (≈ 794 px) wide; image templates are a phone width.
const THUMB_W = 116;
const naturalWidth = computed(() => (props.layout === 'a4' ? 794 : IMAGE_TEMPLATE_WIDTH_PX));
const scale = computed(() => THUMB_W / naturalWidth.value);
const thumbH = computed(() => (props.layout === 'a4' ? Math.round(THUMB_W * 1.414) : Math.round(THUMB_W * 1.7)));
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="px-4 pb-2 pt-4">
      <p class="text-body font-medium">القوالب</p>
      <p class="text-tiny text-text-secondary">{{ layout === 'a4' ? 'فواتير A4 للطباعة' : 'صورة لإرسالها على الموبايل' }}</p>
    </div>

    <div role="radiogroup" :aria-label="layout === 'a4' ? 'قوالب A4' : 'قوالب الصورة'" class="grid flex-1 auto-rows-min grid-cols-2 gap-3 overflow-y-auto px-4 pb-4">
      <button
        v-for="t in templates"
        :key="t.id"
        type="button"
        role="radio"
        :aria-checked="model === t.id"
        :aria-label="t.label"
        :data-template-option="t.id"
        class="group rounded-xl border p-1.5 text-start transition-[border-color,background-color] duration-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary active:scale-[0.98]"
        :class="model === t.id ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
        @click="model = t.id"
      >
        <!-- Thumbnail: the real document, scaled. dir=ltr only positions the scaled box; the document inside stays RTL. -->
        <div class="relative overflow-hidden rounded-md bg-white ring-1 ring-black/5" :style="{ height: `${thumbH}px` }" dir="ltr" aria-hidden="true">
          <div class="pointer-events-none absolute top-0" :style="{ width: `${naturalWidth}px`, transform: `scale(${scale})`, transformOrigin: '0 0', insetInlineStart: '0' }">
            <InvoiceDocument :data="data" :layout="layout" :template-id="t.id" />
          </div>
          <span v-if="model === t.id" class="absolute end-1.5 top-1.5 grid size-5 place-items-center rounded-full bg-primary text-on-primary">
            <Check class="size-3.5" :stroke-width="3" />
          </span>
        </div>
        <span class="mt-1.5 flex flex-wrap items-center gap-x-1 px-0.5 text-label font-medium">
          {{ t.label }}
          <span v-if="t.id === defaultId" class="rounded-full bg-surface-hover px-1.5 text-caption text-text-secondary">الافتراضي</span>
        </span>
        <span class="block px-0.5 text-caption leading-4 text-text-secondary">{{ t.description }}</span>
      </button>
    </div>

    <div v-if="canSetDefault" class="border-t border-border p-3">
      <AppButton class="w-full" size="sm" variant="secondary" :disabled="model === defaultId" :loading="savingDefault" @click="emit('setDefault', model)">
        {{ model === defaultId ? 'هذا هو القالب الافتراضي' : 'تعيين كافتراضي' }}
      </AppButton>
    </div>
  </div>
</template>
