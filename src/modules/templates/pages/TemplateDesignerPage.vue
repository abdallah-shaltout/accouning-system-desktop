<script setup lang="ts">
/**
 * Template designer (Phase 11a) — docs/v2/12-documents-pdf-excel.md §3.
 * Proves brand + header + columns + footer + paper-size options live,
 * end-to-end, against the invoice template only.
 *
 * TemplateOptions groups implemented: Brand (logo position/size, accent
 * color, font family/size), Header (field toggles + title), Columns
 * (toggle only — no drag-reorder), Totals (amount-in-words, balance toggle),
 * Footer (terms, bank details, signature lines, thank-you, page numbers),
 * QR (position, size), Paper (A4/A5/Letter/80mm/58mm).
 *
 * Explicitly SKIPPED for Phase 11a (noted per the task brief, not silently
 * dropped):
 * - Logo upload + crop, stamp/signature image uploads: needs an attachment
 *   picker wired to the (out-of-scope) attachments store; the logo position/
 *   size options still work against the company's existing settings logo.
 * - Party-box field toggles: the shared lib.typ party-box always shows
 *   name/VAT/address — a fixed, reasonable default for now.
 * - Column drag-reorder + relabel: toggle only; reordering needs a drag
 *   library this phase doesn't pull in.
 * - Watermark (مسودة/مدفوعة/نسخة): no reprint-state plumbing yet.
 * - Per-branch "set as default": branches are a later v2 phase; "set as
 *   default" here is per document-kind only.
 * - Previous/current balance totals: fields exist in TemplateOptions and
 *   the Typst layout, but no live customer-balance data source is wired in
 *   (sample payload's `previousBalance`/`currentBalance` are always null).
 */
import { computed, onBeforeUnmount, reactive, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Copy, Download, RotateCcw, Save, Star, Upload } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import PdfPreview from '@/modules/core/components/ui/PdfPreview.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { renderPreview, sampleInvoicePayload, type PreviewError } from '@/modules/core/services/pdfService';
import { saveFile } from '@/modules/core/services/saveFile';
import {
  duplicateTemplate,
  exportTemplate,
  getTemplate,
  importTemplate,
  listTemplates,
  resetTemplateToDefaults,
  saveTemplate,
  setAsDefault,
} from '../services/templateService';
import type { FontFamilyOption, LogoPosition, LogoSize, PaperSize, PdfTemplate, TemplateExport } from '../types';

const route = useRoute('settings-template-designer');
const router = useRouter();
const toast = useToast();

const templateId = computed(() => String(route.params.id ?? ''));
const template = ref<PdfTemplate | null>(null);
const loadError = ref('');

function load() {
  const found = templateId.value ? getTemplate(templateId.value) : undefined;
  if (!found) {
    const all = listTemplates('invoice');
    if (all[0]) {
      router.replace({ name: 'settings-template-designer', params: { id: all[0].id } });
      return;
    }
    loadError.value = 'القالب غير موجود';
    return;
  }
  template.value = JSON.parse(JSON.stringify(found));
}
load();
watch(templateId, load);

const activeTab = ref<'options' | 'advanced'>('options');

// --- Live preview ------------------------------------------------------------------------------
const previewPages = ref<string[]>([]);
const previewLoading = ref(false);
const compileErrors = ref<{ line: number | null; column: number | null; severity: string; message: string }[]>([]);
const samplePayload = sampleInvoicePayload();

let debounceHandle: ReturnType<typeof setTimeout> | null = null;
function schedulePreview() {
  if (debounceHandle) clearTimeout(debounceHandle);
  debounceHandle = setTimeout(runPreview, 300);
}
onBeforeUnmount(() => {
  if (debounceHandle) clearTimeout(debounceHandle);
});

async function runPreview() {
  if (!template.value) return;
  previewLoading.value = true;
  compileErrors.value = [];
  try {
    const result = await renderPreview(samplePayload, template.value.options, template.value.customSource, template.value.baseTemplateId);
    previewPages.value = result.pages;
  } catch (err) {
    const diagnostics = (err as PreviewError)?.diagnostics ?? [{ line: null, column: null, severity: 'error', message: String(err) }];
    compileErrors.value = diagnostics;
    previewPages.value = [];
  } finally {
    previewLoading.value = false;
  }
}

watch(
  () => template.value && JSON.stringify(template.value.options),
  () => schedulePreview(),
);
watch(
  () => template.value?.customSource,
  () => schedulePreview(),
);
watch(template, (t) => {
  if (t) schedulePreview();
}, { immediate: true });

// --- Top bar actions -----------------------------------------------------------------------------
function persist() {
  if (!template.value) return;
  saveTemplate(template.value);
  toast.success('تم حفظ القالب');
}

function markDefault() {
  if (!template.value) return;
  setAsDefault(template.value.id);
  template.value.isDefault = true;
  toast.success('تم ضبط القالب كافتراضي');
}

function duplicate() {
  if (!template.value) return;
  persist();
  const copy = duplicateTemplate(template.value.id);
  if (copy) router.push({ name: 'settings-template-designer', params: { id: copy.id } });
}

function resetToDefaults() {
  if (!template.value) return;
  const reset = resetTemplateToDefaults(template.value.id);
  if (reset) template.value = JSON.parse(JSON.stringify(reset));
  toast.info('تمت إعادة القالب إلى الإعدادات الافتراضية');
}

async function exportJson() {
  if (!template.value) return;
  const payload = exportTemplate(template.value);
  await saveFile(JSON.stringify(payload, null, 2), { suggestedName: `${template.value.name}.json`, kind: 'json' });
}

const fileInput = ref<HTMLInputElement>();
function triggerImport() {
  fileInput.value?.click();
}
async function onImportFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (!file) return;
  try {
    const text = await file.text();
    const json = JSON.parse(text) as TemplateExport;
    const imported = importTemplate(json);
    toast.success('تم استيراد القالب');
    router.push({ name: 'settings-template-designer', params: { id: imported.id } });
  } catch (err) {
    toast.error(err, 'تعذر استيراد القالب');
  } finally {
    (e.target as HTMLInputElement).value = '';
  }
}

// --- Advanced tab: source editor -----------------------------------------------------------------
function openAdvanced() {
  if (!template.value) return;
  if (template.value.customSource === null) {
    // Seed the editor with the effective built-in template so editing starts
    // from something real, not a blank file.
    template.value.customSource = BUILTIN_SOURCE[template.value.baseTemplateId];
  }
  activeTab.value = 'advanced';
}
function backToGenerated() {
  if (!template.value) return;
  template.value.customSource = null;
  activeTab.value = 'options';
}

// v2 phase 11b: the 9 new document kinds' built-in templates aren't a single reusable function
// call the way `invoice-document` is (each has its own bespoke layout in
// src-tauri/templates/<id>.typ, built from lib.typ's smaller pieces). The kinds built from
// lib.typ's shared header/party-box/lines-table/totals/footer pieces (quotation, credit/debit
// note, purchase order, transfer note) get an equivalent reconstruction here; the kinds with a
// fully bespoke layout (voucher, statement, Z-report, generic report, both label layouts) get a
// short stub pointing at the real file instead of a stale near-copy that could drift from it.
const BUILTIN_SOURCE: Record<PdfTemplate['baseTemplateId'], string> = {
  invoice_standard: '#import "lib.typ": invoice-document\n\n#let data = json("data.json")\n#let opts = json("opts.json")\n\n#invoice-document(data, opts, simplified: false)\n',
  invoice_simplified: '#import "lib.typ": invoice-document\n\n#let data = json("data.json")\n#let opts = json("opts.json")\n\n#invoice-document(data, opts, simplified: true)\n',
  quotation: '#import "lib.typ": header, party-box, lines-table, totals-block, footer-block, page-margin, page-typst-size, font-family, font-size\n\n#let data = json("data.json")\n#let opts = json("opts.json")\n\n#set page(paper: page-typst-size(opts), margin: page-margin(opts))\n#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)\n\n#header(data, opts)\n#party-box(data, opts)\n#lines-table(data, opts)\n#totals-block(data, opts)\n#footer-block(data, opts)\n',
  credit_note: '#import "lib.typ": header, party-box, lines-table, totals-block, qr-block, footer-block, page-margin, page-typst-size, font-family, font-size\n\n#let data = json("data.json")\n#let opts = json("opts.json")\n\n#set page(paper: page-typst-size(opts), margin: page-margin(opts))\n#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)\n\n#header(data, opts)\n#party-box(data, opts)\n#lines-table(data, opts)\n#totals-block(data, opts)\n#footer-block(data, opts)\n',
  debit_note: '#import "lib.typ": header, party-box, lines-table, totals-block, footer-block, page-margin, page-typst-size, font-family, font-size\n\n#let data = json("data.json")\n#let opts = json("opts.json")\n\n#set page(paper: page-typst-size(opts), margin: page-margin(opts))\n#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)\n\n#header(data, opts)\n#party-box(data, opts)\n#lines-table(data, opts)\n#totals-block(data, opts)\n#footer-block(data, opts)\n',
  purchase_order: '#import "lib.typ": header, party-box, lines-table, totals-block, footer-block, page-margin, page-typst-size, font-family, font-size\n\n#let data = json("data.json")\n#let opts = json("opts.json")\n\n#set page(paper: page-typst-size(opts), margin: page-margin(opts))\n#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)\n\n#header(data, opts)\n#party-box(data, opts)\n#lines-table(data, opts)\n#totals-block(data, opts)\n#footer-block(data, opts)\n',
  voucher: '// The voucher template (src-tauri/templates/voucher.typ) has its own header/amount-box\n// layout rather than reusing lib.typ\'s invoice header — open that file for the full source.\n#let data = json("data.json")\n#let opts = json("opts.json")\n',
  statement: '// The statement template (src-tauri/templates/statement.typ) has its own running-balance\n// ledger table rather than lib.typ\'s invoice lines-table — open that file for the full source.\n#let data = json("data.json")\n#let opts = json("opts.json")\n',
  z_report: '// The Z-report template (src-tauri/templates/z_report.typ) has its own key/value summary\n// grid — open that file for the full source.\n#let data = json("data.json")\n#let opts = json("opts.json")\n',
  transfer_note: '#import "lib.typ": header, lines-table, footer-block, page-margin, page-typst-size, font-family, font-size\n\n#let data = json("data.json")\n#let opts = json("opts.json")\n\n#set page(paper: page-typst-size(opts), margin: page-margin(opts))\n#set text(font: font-family(opts), lang: "ar", region: "sa", size: font-size(opts), dir: rtl)\n\n#header(data, opts)\n#lines-table(data, opts)\n#footer-block(data, opts)\n',
  generic_report: '// The generic report template (src-tauri/templates/generic_report.typ) reads arbitrary\n// row/column data rather than the invoice line shape — open that file for the full source.\n#let data = json("data.json")\n#let opts = json("opts.json")\n',
  label_sheet: '// The label sheet template (src-tauri/templates/label_sheet.typ) lays out a grid of labels,\n// not a single document body — open that file for the full source.\n#let data = json("data.json")\n#let opts = json("opts.json")\n',
  label_thermal: '// The thermal label template (src-tauri/templates/label_thermal.typ) is one label per page —\n// open that file for the full source.\n#let data = json("data.json")\n#let opts = json("opts.json")\n',
};

// --- Options helpers -------------------------------------------------------------------------------
const logoPositions: { value: LogoPosition; label: string }[] = [
  { value: 'start', label: 'البداية' },
  { value: 'center', label: 'الوسط' },
  { value: 'end', label: 'النهاية' },
];
const logoSizes: { value: LogoSize; label: string }[] = [
  { value: 's', label: 'صغير' },
  { value: 'm', label: 'متوسط' },
  { value: 'l', label: 'كبير' },
];
const fontFamilies: { value: FontFamilyOption; label: string }[] = [
  { value: 'Cairo', label: 'Cairo' },
  { value: 'Noto Naskh Arabic', label: 'Noto Naskh Arabic' },
  { value: 'IBM Plex Sans Arabic', label: 'IBM Plex Sans Arabic' },
  { value: 'Tajawal', label: 'Tajawal' },
];
const paperSizes: { value: PaperSize; label: string }[] = [
  { value: 'a4', label: 'A4' },
  { value: 'a5', label: 'A5' },
  { value: 'letter', label: 'Letter' },
  { value: '80mm', label: '80 مم (حراري)' },
  { value: '58mm', label: '58 مم (حراري)' },
];
const accentPresets = ['#4f46e5', '#0d9488', '#e11d48', '#b45309', '#1f2937'];

const accordions = reactive({ brand: true, header: false, columns: false, totals: false, footer: false, qr: false, paper: false });
function toggle(key: keyof typeof accordions) {
  accordions[key] = !accordions[key];
}
</script>

<template>
  <div v-if="loadError">
    <PageHeader title="قالب الطباعة" :back="{ name: 'settings-templates' }" />
    <p class="text-body text-danger">{{ loadError }}</p>
  </div>
  <div v-else-if="template" class="flex h-[calc(100vh-6rem)] flex-col">
    <!-- Top bar -->
    <PageHeader :title="template.name" :back="{ name: 'settings-templates' }">
      <template #subtitle>
        <span v-if="template.isDefault" class="inline-flex items-center gap-1 text-xs text-primary"><Star class="size-3 fill-current" /> القالب الافتراضي</span>
        <span v-else class="text-xs text-text-secondary">فاتورة ضريبية</span>
      </template>
      <template #actions>
        <AppInput v-model="template.name" placeholder="اسم القالب" class="w-56" />
        <AppButton :icon="Star" :disabled="template.isDefault" @click="markDefault">تعيين كافتراضي</AppButton>
        <AppButton :icon="Copy" @click="duplicate">نسخ</AppButton>
        <AppButton :icon="RotateCcw" @click="resetToDefaults">إعادة تعيين</AppButton>
        <AppButton :icon="Download" @click="exportJson">تصدير</AppButton>
        <AppButton :icon="Upload" @click="triggerImport">استيراد</AppButton>
        <input ref="fileInput" type="file" accept="application/json" class="hidden" @change="onImportFile" />
        <AppButton variant="primary" :icon="Save" @click="persist">حفظ</AppButton>
      </template>
    </PageHeader>

    <SegmentedControl
      v-model="activeTab"
      class="mb-4"
      :options="[
        { value: 'options', label: 'الخيارات' },
        { value: 'advanced', label: 'متقدم: كود Typst' },
      ]"
      @update:model-value="(v) => (v === 'advanced' ? openAdvanced() : undefined)"
    />

    <div class="grid min-h-0 flex-1 gap-4 lg:grid-cols-[360px_1fr]">
      <!-- Options / advanced panel -->
      <div class="min-h-0 overflow-y-auto pe-1">
        <div v-if="activeTab === 'options'" class="space-y-3">
          <!-- Brand -->
          <AppCard padding="none">
            <button type="button" class="flex w-full items-center justify-between px-4 py-3 text-body font-medium" @click="toggle('brand')">
              العلامة التجارية
              <span class="text-xs text-text-secondary">{{ accordions.brand ? '−' : '+' }}</span>
            </button>
            <div v-if="accordions.brand" class="space-y-3 border-t border-border p-4">
              <div>
                <label class="field-label">موضع الشعار</label>
                <SegmentedControl v-model="template.options.logoPosition" size="sm" :options="logoPositions" />
              </div>
              <div>
                <label class="field-label">حجم الشعار</label>
                <SegmentedControl v-model="template.options.logoSize" size="sm" :options="logoSizes" />
              </div>
              <div>
                <label class="field-label">لون التمييز</label>
                <div class="flex items-center gap-2">
                  <input v-model="template.options.accentColor" type="color" class="h-8 w-10 cursor-pointer rounded border border-border" />
                  <AppInput v-model="template.options.accentColor" ltr class="flex-1" />
                </div>
                <div class="mt-1.5 flex gap-1.5">
                  <button
                    v-for="c in accentPresets"
                    :key="c"
                    type="button"
                    class="size-5 rounded-full border border-border"
                    :style="{ backgroundColor: c }"
                    :aria-label="c"
                    @click="template.options.accentColor = c"
                  />
                </div>
              </div>
              <AppSelect v-model="template.options.fontFamily" label="الخط" :options="fontFamilies" />
              <AppInput v-model.number="template.options.fontSize" type="number" label="حجم الخط (نقطة)" min="7" max="16" />
            </div>
          </AppCard>

          <!-- Header -->
          <AppCard padding="none">
            <button type="button" class="flex w-full items-center justify-between px-4 py-3 text-body font-medium" @click="toggle('header')">
              الترويسة
              <span class="text-xs text-text-secondary">{{ accordions.header ? '−' : '+' }}</span>
            </button>
            <div v-if="accordions.header" class="space-y-2.5 border-t border-border p-4">
              <AppInput v-model="template.options.header.title" label="عنوان المستند (عربي)" />
              <AppInput v-model="template.options.header.titleEn" label="عنوان المستند (إنجليزي)" ltr />
              <AppSwitch v-model="template.options.header.showCompanyName" label="اسم الشركة" />
              <AppSwitch v-model="template.options.header.showAddress" label="العنوان" />
              <AppSwitch v-model="template.options.header.showVatNumber" label="الرقم الضريبي" />
              <AppSwitch v-model="template.options.header.showCommercialRegister" label="السجل التجاري" />
              <AppSwitch v-model="template.options.header.showPhone" label="الهاتف" />
              <AppSwitch v-model="template.options.header.showEmail" label="البريد الإلكتروني" />
              <AppSwitch v-model="template.options.header.showWebsite" label="الموقع الإلكتروني" />
            </div>
          </AppCard>

          <!-- Columns -->
          <AppCard padding="none">
            <button type="button" class="flex w-full items-center justify-between px-4 py-3 text-body font-medium" @click="toggle('columns')">
              أعمدة الجدول
              <span class="text-xs text-text-secondary">{{ accordions.columns ? '−' : '+' }}</span>
            </button>
            <div v-if="accordions.columns" class="space-y-1.5 border-t border-border p-4">
              <AppSwitch v-for="col in template.options.columns" :key="col.key" v-model="col.visible" :label="col.label" />
              <p class="pt-1 text-xs text-text-secondary">إعادة الترتيب بالسحب غير متاحة بعد (المرحلة 11ب).</p>
            </div>
          </AppCard>

          <!-- Totals -->
          <AppCard padding="none">
            <button type="button" class="flex w-full items-center justify-between px-4 py-3 text-body font-medium" @click="toggle('totals')">
              الإجماليات
              <span class="text-xs text-text-secondary">{{ accordions.totals ? '−' : '+' }}</span>
            </button>
            <div v-if="accordions.totals" class="space-y-2.5 border-t border-border p-4">
              <AppSwitch v-model="template.options.totals.showAmountInWords" label="المبلغ كتابة (تفقيط)" />
              <AppSwitch v-model="template.options.totals.showBalance" label="الرصيد السابق / الحالي" />
            </div>
          </AppCard>

          <!-- Footer -->
          <AppCard padding="none">
            <button type="button" class="flex w-full items-center justify-between px-4 py-3 text-body font-medium" @click="toggle('footer')">
              التذييل
              <span class="text-xs text-text-secondary">{{ accordions.footer ? '−' : '+' }}</span>
            </button>
            <div v-if="accordions.footer" class="space-y-2.5 border-t border-border p-4">
              <AppTextarea v-model="template.options.footer.terms" label="الشروط والأحكام" :rows="2" />
              <AppInput v-model="template.options.footer.bankDetails" label="البيانات البنكية (IBAN)" ltr />
              <AppSwitch v-model="template.options.footer.showSignatureLines" label="خطوط التوقيع (المستلم / المحاسب / المدير)" />
              <AppInput v-model="template.options.footer.thankYouLine" label="عبارة الشكر" />
              <AppSwitch v-model="template.options.footer.showPageNumbers" label="ترقيم الصفحات" />
            </div>
          </AppCard>

          <!-- QR -->
          <AppCard padding="none">
            <button type="button" class="flex w-full items-center justify-between px-4 py-3 text-body font-medium" @click="toggle('qr')">
              رمز الاستجابة السريعة
              <span class="text-xs text-text-secondary">{{ accordions.qr ? '−' : '+' }}</span>
            </button>
            <div v-if="accordions.qr" class="space-y-2.5 border-t border-border p-4">
              <div>
                <label class="field-label">الموضع</label>
                <SegmentedControl
                  v-model="template.options.qr.position"
                  size="sm"
                  :options="[
                    { value: 'start', label: 'البداية' },
                    { value: 'center', label: 'الوسط' },
                    { value: 'end', label: 'النهاية' },
                  ]"
                />
              </div>
              <AppInput v-model="template.options.qr.size" label="الحجم (مثال: 3cm)" ltr />
            </div>
          </AppCard>

          <!-- Paper -->
          <AppCard padding="none">
            <button type="button" class="flex w-full items-center justify-between px-4 py-3 text-body font-medium" @click="toggle('paper')">
              الورق
              <span class="text-xs text-text-secondary">{{ accordions.paper ? '−' : '+' }}</span>
            </button>
            <div v-if="accordions.paper" class="border-t border-border p-4">
              <AppSelect v-model="template.options.paper" label="حجم الورق" :options="paperSizes" />
            </div>
          </AppCard>
        </div>

        <!-- Advanced: Typst source editor -->
        <div v-else class="space-y-3">
          <AppButton size="sm" variant="ghost" @click="backToGenerated">العودة للقالب المولّد</AppButton>
          <textarea
            v-model="template.customSource"
            dir="ltr"
            spellcheck="false"
            class="control h-[480px] w-full resize-none font-mono text-xs leading-relaxed"
            placeholder="Typst source"
          />
          <div v-if="compileErrors.length" class="space-y-1.5 rounded-md border border-danger/40 bg-danger/5 p-3">
            <p v-for="(err, i) in compileErrors" :key="i" class="text-xs text-danger">
              <span v-if="err.line" class="num font-medium">سطر {{ err.line }}<template v-if="err.column">, عمود {{ err.column }}</template>: </span>
              {{ err.message }}
            </p>
          </div>
          <p v-else-if="!previewLoading" class="text-xs text-success">لا توجد أخطاء ترجمة.</p>
        </div>
      </div>

      <!-- Live preview -->
      <AppCard padding="none" class="min-h-0 overflow-hidden">
        <PdfPreview :pages="previewPages" :loading="previewLoading" empty-message="تعذر إنشاء المعاينة" />
      </AppCard>
    </div>
  </div>
</template>
