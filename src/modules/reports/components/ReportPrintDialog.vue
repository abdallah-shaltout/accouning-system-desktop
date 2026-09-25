<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { FileDown, Printer, X } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { saveReportPdf } from '@/modules/core/services/pdfService';
import { DEFAULT_SIGNATURES } from '../print/build';
import { renderReportHtml } from '../print/renderHtml';
import type { ReportDocument } from '../print/types';

/**
 * Print preview for an official report: shows the rendered document (not the report screen), with
 * the two choices that change the paper — orientation and the signature row — then prints it from
 * this same frame, or saves a native PDF (desktop: `report.typ` via Typst; browser: the print
 * dialog's "Save as PDF").
 */
const props = defineProps<{ doc: ReportDocument | null; fileName: string }>();
const open = defineModel<boolean>('open', { default: false });

const toast = useToast();
const frame = ref<HTMLIFrameElement>();
const withSignatures = ref(false);
const orientation = ref<'portrait' | 'landscape'>('portrait');
const saving = ref(false);
const desktop = isTauri();

watch(
  () => props.doc,
  (d) => {
    if (!d) return;
    withSignatures.value = d.signatures.length > 0;
    orientation.value = d.orientation;
  },
  { immediate: true },
);

const effective = computed<ReportDocument | null>(() => {
  const d = props.doc;
  if (!d) return null;
  return { ...d, orientation: orientation.value, signatures: withSignatures.value ? (d.signatures.length ? d.signatures : DEFAULT_SIGNATURES) : [] };
});
const html = computed(() => (effective.value ? renderReportHtml(effective.value, 'screen') : ''));

async function print() {
  const win = frame.value?.contentWindow;
  if (!win) return;
  try {
    await win.document.fonts?.ready;
  } catch {
    /* fonts API unavailable — print anyway */
  }
  win.focus();
  win.print();
}

async function savePdf() {
  if (!effective.value) return;
  if (!desktop) {
    toast.info('اختر «حفظ كـ PDF» كطابعة في نافذة الطباعة');
    await print();
    return;
  }
  saving.value = true;
  try {
    if (await saveReportPdf(effective.value, `${props.fileName}.pdf`)) toast.success('تم حفظ التقرير', `${props.fileName}.pdf`);
  } catch (err) {
    toast.error(err, 'تعذر إنشاء ملف PDF');
  } finally {
    saving.value = false;
  }
}

function onKeydown(e: KeyboardEvent) {
  if (!open.value) return;
  if (e.key === 'Escape') {
    e.stopPropagation();
    open.value = false;
  } else if ((e.ctrlKey || e.metaKey) && e.code === 'KeyP') {
    e.preventDefault();
    e.stopPropagation();
    void print();
  }
}
watch(open, (v) => (v ? window.addEventListener('keydown', onKeydown, true) : window.removeEventListener('keydown', onKeydown, true)), { immediate: true });
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown, true));
</script>

<template>
  <Teleport to="body">
    <Transition enter-active-class="transition duration-150 ease-out" enter-from-class="opacity-0" leave-active-class="transition duration-100 ease-in" leave-to-class="opacity-0">
      <div v-if="open && effective" dir="rtl" class="fixed inset-0 z-50 flex items-center justify-center bg-black/55 p-3 sm:p-5" @mousedown.self="open = false">
        <div role="dialog" aria-modal="true" :aria-label="`معاينة الطباعة — ${effective.title}`" class="flex h-full max-h-[96vh] w-full max-w-6xl flex-col overflow-hidden rounded-xl border border-border bg-background shadow-2xl" data-testid="report-print-dialog">
          <header class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-border px-4 py-3">
            <div class="me-auto min-w-0">
              <h2 class="truncate text-lead font-semibold">معاينة الطباعة</h2>
              <p class="truncate text-xs text-text-secondary">{{ effective.title }} — نسخة رسمية بترويسة المنشأة</p>
            </div>
            <SegmentedControl
              v-model="orientation"
              size="sm"
              :options="[
                { value: 'portrait', label: 'طولي' },
                { value: 'landscape', label: 'عرضي' },
              ]"
            />
            <AppSwitch v-model="withSignatures" label="خانات التوقيع" />
            <div class="flex items-center gap-2">
              <AppButton variant="primary" size="sm" :icon="Printer" kbd="Ctrl+P" data-testid="report-print" @click="print">طباعة</AppButton>
              <AppButton size="sm" :icon="FileDown" :loading="saving" data-testid="report-pdf" @click="savePdf">حفظ PDF</AppButton>
              <button type="button" aria-label="إغلاق" class="rounded-md p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" @click="open = false">
                <X class="size-4" />
              </button>
            </div>
          </header>
          <iframe ref="frame" :srcdoc="html" title="معاينة التقرير" class="min-h-0 w-full flex-1 border-0 bg-surface-hover" data-testid="report-print-frame" />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
