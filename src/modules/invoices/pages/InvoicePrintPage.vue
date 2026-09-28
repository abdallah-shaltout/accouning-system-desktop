<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Copy, Download, Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useHotkeys } from '@/modules/core/controllers/useHotkeys';
import { useToast } from '@/modules/core/controllers/useToast';
import { dirIcon } from '@/modules/core/helpers/dirIcon';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import type { ThermalWidth } from '@/modules/settings/types';
import type { AppRoute } from '@/modules/core/types/route';
import InvoiceDocument from '../components/InvoiceDocument.vue';
import TemplateGallery from '../components/TemplateGallery.vue';
import { usePrintTemplates } from '../controllers/usePrintTemplates';
import type { PrintLayout } from '../helpers/invoiceTemplates';
import { copyInvoiceImage, saveInvoiceImage } from '../services/invoiceImageService';
import { getInvoicePrintData } from '../services/invoiceService';

/**
 * Chrome-free print preview. Layout defaults to StoreSettings.printer and can be switched here for a
 * one-off print; A4 and the mobile image layout pick a template from the gallery (plan 22).
 * `?auto=1` opens the print dialog as soon as the document renders (POS); `?template=` preselects one.
 */
const route = useRoute('invoice-print');
const router = useRouter();
const settings = useSettingsStore();
const toast = useToast();
const id = String(route.params.id);

const asLayout = (m: unknown): PrintLayout | undefined => (m === 'a4' || m === 'thermal' || m === 'image' ? m : undefined);
const mode = ref<PrintLayout>(asLayout(route.query.mode) ?? settings.settings?.printer.mode ?? 'a4');
const width = ref<ThermalWidth>(Number(route.query.width) === 58 ? 58 : Number(route.query.width) === 80 ? 80 : settings.settings?.printer.thermalWidthMm ?? 80);
const { active, activeDefault, activeMeta, canSetDefault, savingDefault, setDefault } = usePrintTemplates(mode, () => route.query.template);

const { data, error, reload } = useAsync(() => getInvoicePrintData(id));

// The same route can be re-entered with a different ?mode / ?width (component is reused).
watch(
  () => [route.query.mode, route.query.width],
  ([m, w]) => {
    const layout = asLayout(m);
    if (layout) mode.value = layout;
    if (w === '58' || w === '80') width.value = Number(w) as ThermalWidth;
  },
);

// Page size for the browser/WebView print dialog. Edge-to-edge templates print with no page margin.
const pageStyle = document.createElement('style');
document.head.appendChild(pageStyle);
watch(
  [mode, width, activeMeta],
  () => {
    if (mode.value === 'thermal') pageStyle.textContent = `@page { size: ${width.value}mm auto; margin: 0; }`;
    else if (mode.value === 'image') pageStyle.textContent = '@page { margin: 0; }';
    else pageStyle.textContent = `@page { size: A4; margin: ${activeMeta.value?.bleed ? '0' : '12mm'}; }`;
  },
  { immediate: true },
);
onBeforeUnmount(() => pageStyle.remove());

function print() {
  window.print();
}

watch(data, (d) => {
  if (d && route.query.auto === '1') setTimeout(print, 300);
});

// Mobile image: save as PNG (native Save dialog) or copy for pasting into a chat.
const doc = ref<InstanceType<typeof InvoiceDocument>>();
const exporting = ref<'save' | 'copy' | null>(null);
async function exportImage(kind: 'save' | 'copy') {
  const el = doc.value?.root;
  if (!el || !data.value) return;
  exporting.value = kind;
  try {
    if (kind === 'save') await saveInvoiceImage(el, data.value.invoice.number);
    else {
      await copyInvoiceImage(el);
      toast.success('تم نسخ صورة الفاتورة', 'الصقها في المحادثة مباشرة');
    }
  } catch (err) {
    toast.error(err, kind === 'save' ? 'تعذر حفظ الصورة' : 'تعذر نسخ الصورة');
  } finally {
    exporting.value = null;
  }
}

const backTo = computed<AppRoute>(() => (route.query.from === 'pos' ? { name: 'pos' } : id === 'sample' ? { name: 'settings-printing' } : { name: 'invoice', params: { id } }));
function close() {
  router.push(backTo.value);
}

useHotkeys({ 'ctrl+p': { id: 'print.document', label: 'طباعة المستند', group: 'الطباعة', handler: print }, Escape: close });
</script>

<template>
  <div class="flex min-h-screen flex-col bg-surface">
    <div class="no-print sticky top-0 z-10 flex h-12 items-center justify-between gap-3 border-b border-border bg-background px-4">
      <div class="flex items-center gap-2">
        <AppButton size="sm" variant="ghost" :icon="dirIcon.back" icon-rtl-flip @click="close">رجوع</AppButton>
        <span class="text-body font-medium">معاينة الطباعة</span>
        <span v-if="data" class="num text-body text-text-secondary">{{ data.invoice.number }}</span>
      </div>
      <div class="flex items-center gap-2">
        <SegmentedControl
          v-model="mode"
          size="sm"
          :options="[
            { value: 'a4', label: 'A4' },
            { value: 'thermal', label: 'حراري' },
            { value: 'image', label: 'صورة للموبايل' },
          ]"
        />
        <SegmentedControl
          v-if="mode === 'thermal'"
          v-model="width"
          size="sm"
          :options="[
            { value: 80, label: '80mm' },
            { value: 58, label: '58mm' },
          ]"
        />
        <template v-if="mode === 'image'">
          <AppButton size="sm" variant="secondary" :icon="Copy" :loading="exporting === 'copy'" :disabled="!data || !!exporting" @click="exportImage('copy')">نسخ الصورة</AppButton>
          <AppButton size="sm" variant="primary" :icon="Download" :loading="exporting === 'save'" :disabled="!data || !!exporting" @click="exportImage('save')">حفظ كصورة</AppButton>
        </template>
        <AppButton size="sm" :variant="mode === 'image' ? 'secondary' : 'primary'" :icon="Printer" kbd="Ctrl+P" :disabled="!data" @click="print">طباعة</AppButton>
      </div>
    </div>

    <ErrorState v-if="error" :message="error" @retry="reload" />
    <div v-else class="flex flex-1">
      <aside v-if="data && mode !== 'thermal'" class="no-print sticky top-12 h-[calc(100vh-3rem)] w-72 shrink-0 border-e border-border bg-background" data-testid="template-gallery">
        <TemplateGallery
          v-model="active"
          :data="data"
          :layout="mode"
          :default-id="activeDefault"
          :can-set-default="canSetDefault"
          :saving-default="savingDefault"
          @set-default="setDefault"
        />
      </aside>
      <div class="flex flex-1 justify-center overflow-x-auto p-8 print:p-0">
        <div v-if="!data" class="w-[186mm] rounded-lg bg-white p-10"><SkeletonBlock :lines="14" /></div>
        <div v-else class="print-root h-fit shadow-lg ring-1 ring-black/5 print:shadow-none print:ring-0" :class="mode === 'image' ? 'overflow-hidden rounded-3xl print:rounded-none' : ''">
          <InvoiceDocument ref="doc" :data="data" :layout="mode" :template-id="active" :width="width" />
        </div>
      </div>
    </div>
  </div>
</template>
