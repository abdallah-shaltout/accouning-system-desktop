<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useHotkeys } from '@/modules/core/controllers/useHotkeys';
import { dirIcon } from '@/modules/core/helpers/dirIcon';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import type { PrinterMode, ThermalWidth } from '@/modules/settings/types';
import InvoiceA4 from '../components/InvoiceA4.vue';
import InvoiceThermal from '../components/InvoiceThermal.vue';
import { getInvoicePrintData } from '../services/invoiceService';

/**
 * Chrome-free print preview. Layout defaults to StoreSettings.printer and can be switched here
 * for a one-off print. `?auto=1` opens the print dialog as soon as the document renders (POS).
 */
const route = useRoute('invoice-print');
const router = useRouter();
const settings = useSettingsStore();
const id = String(route.params.id);

const mode = ref<PrinterMode>((route.query.mode as PrinterMode) ?? settings.settings?.printer.mode ?? 'a4');
const width = ref<ThermalWidth>(Number(route.query.width) === 58 ? 58 : Number(route.query.width) === 80 ? 80 : settings.settings?.printer.thermalWidthMm ?? 80);

const { data, error, reload } = useAsync(() => getInvoicePrintData(id));

// The same route can be re-entered with a different ?mode / ?width (component is reused).
watch(
  () => [route.query.mode, route.query.width],
  ([m, w]) => {
    if (m === 'a4' || m === 'thermal') mode.value = m;
    if (w === '58' || w === '80') width.value = Number(w) as ThermalWidth;
  },
);

// Page size for the browser/WebView print dialog.
const pageStyle = document.createElement('style');
document.head.appendChild(pageStyle);
watch(
  [mode, width],
  () => {
    pageStyle.textContent = mode.value === 'a4' ? '@page { size: A4; margin: 12mm; }' : `@page { size: ${width.value}mm auto; margin: 0; }`;
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

const backTo = computed(() => (typeof route.query.back === 'string' ? route.query.back : id === 'sample' ? '/settings/printing' : `/invoices/${id}`));
function close() {
  router.push(backTo.value);
}

useHotkeys({ 'ctrl+p': { id: 'print.document', label: 'طباعة المستند', group: 'الطباعة', handler: print }, Escape: close });
</script>

<template>
  <div class="min-h-screen bg-surface">
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
        <AppButton size="sm" variant="primary" :icon="Printer" kbd="Ctrl+P" :disabled="!data" @click="print">طباعة</AppButton>
      </div>
    </div>

    <ErrorState v-if="error" :message="error" @retry="reload" />
    <div v-else class="flex justify-center p-8 print:p-0">
      <div v-if="!data" class="w-[186mm] rounded-lg bg-white p-10"><SkeletonBlock :lines="14" /></div>
      <div v-else class="print-root shadow-lg ring-1 ring-black/5 print:shadow-none print:ring-0" :class="mode === 'a4' ? 'bg-white p-[12mm] print:p-0' : ''">
        <InvoiceA4 v-if="mode === 'a4'" :data="data" />
        <InvoiceThermal v-else :data="data" :width="width" />
      </div>
    </div>
  </div>
</template>
