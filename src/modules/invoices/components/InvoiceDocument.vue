<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { ThermalWidth } from '@/modules/settings/types';
import { DOC_FONT_CLASS, templateMeta, type PrintLayout, type TemplateId } from '../helpers/invoiceTemplates';
import type { PrintData } from '../services/invoiceService';
import InvoiceThermal from './InvoiceThermal.vue';
import { loadDocFont, templateComponent } from './templates/registry';

/**
 * Plan 22: renders one invoice in a layout — the fixed thermal receipt, or an A4 / image template
 * from the registry in that template's face. The print page, the settings preview and the gallery
 * thumbnails all go through this, so a template looks the same everywhere.
 */
const props = withDefaults(defineProps<{ data: PrintData; layout: PrintLayout; templateId: TemplateId; width?: ThermalWidth }>(), { width: 80 });

const root = ref<HTMLElement>();
const meta = computed(() => templateMeta(props.templateId));
const component = computed(() => templateComponent(props.templateId));

watch(
  meta,
  (m) => {
    if (m && props.layout !== 'thermal') void loadDocFont(m.font);
  },
  { immediate: true },
);

defineExpose({ root });
</script>

<template>
  <div ref="root" class="w-fit" :data-template="layout === 'thermal' ? 'thermal' : templateId">
    <InvoiceThermal v-if="layout === 'thermal'" :data="data" :width="width" />
    <!-- The standard A4 template draws inside a 12 mm page margin; every other template is edge to edge. -->
    <div v-else :class="[meta ? DOC_FONT_CLASS[meta.font] : '', layout === 'a4' && !meta?.bleed ? 'bg-white p-[12mm] print:p-0' : '']">
      <component :is="component" :data="data" />
    </div>
  </div>
</template>
