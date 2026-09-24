<script setup lang="ts">
/**
 * v2 phase 5 (docs/v2/05-onboarding.md §5): the generic Excel-import wizard modal. Driven entirely
 * by an `ImportDescriptor` (see `./types.ts`) — download template → upload → map columns →
 * validate → import with progress + summary. Used by the opening-balances stock/customers tabs and
 * (via a small trigger button) the customer list.
 */
import { computed, ref } from 'vue';
import { Check, Download, TriangleAlert, Upload } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { autoMapHeaders, downloadErrorFile, downloadImportTemplate, loadSavedMapping, parseUploadedFile, saveMapping } from './importXlsx';
import type { ImportCommitSummary, ImportDescriptor, ImportRowResult } from './types';

const props = defineProps<{ descriptor: ImportDescriptor }>();
const emit = defineEmits<{ close: []; imported: [rows: any[]] }>();

type Stage = 'upload' | 'map' | 'validate' | 'importing' | 'done';
const stage = ref<Stage>('upload');
const fileName = ref('');
const headers = ref<string[]>([]);
const rawRows = ref<Record<string, unknown>[]>([]);
const mapping = ref<Record<string, string>>({}); // header -> column key
const parsedRows = ref<ImportRowResult[]>([]);
const summary = ref<ImportCommitSummary | null>(null);
const progress = ref({ done: 0, total: 0 });
const dragOver = ref(false);
const loadError = ref('');

async function onDownloadTemplate() {
  await downloadImportTemplate(props.descriptor);
}

async function onFile(file: File | undefined) {
  if (!file) return;
  loadError.value = '';
  try {
    fileName.value = file.name;
    const parsed = await parseUploadedFile(file);
    headers.value = parsed.headers;
    rawRows.value = parsed.rows;
    const saved = loadSavedMapping(props.descriptor.key);
    const autoMapped = autoMapHeaders(parsed.headers, props.descriptor.columns);
    const map: Record<string, string> = {};
    parsed.headers.forEach((h, i) => {
      const fromSaved = saved?.[h];
      const fromAuto = autoMapped[i];
      if (fromSaved && props.descriptor.columns.some((c) => c.key === fromSaved)) map[h] = fromSaved;
      else if (fromAuto) map[h] = fromAuto;
    });
    mapping.value = map;
    stage.value = 'map';
  } catch (err) {
    loadError.value = errorMessage(err);
  }
}

function onDrop(e: DragEvent) {
  dragOver.value = false;
  const file = e.dataTransfer?.files?.[0];
  void onFile(file);
}

function onPick(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0];
  void onFile(file);
}

const unmappedRequired = computed(() => {
  const mappedKeys = new Set(Object.values(mapping.value));
  return props.descriptor.columns.filter((c) => c.required && !mappedKeys.has(c.key));
});

function confirmMapping() {
  saveMapping(props.descriptor.key, mapping.value);
  validateAll();
  stage.value = 'validate';
}

function validateAll() {
  const headerByColumnKey = new Map(Object.entries(mapping.value).map(([h, k]) => [k, h]));
  parsedRows.value = rawRows.value.map((raw, index) => {
    const row: Record<string, unknown> = {};
    const errors: Record<string, string> = {};
    const warnings: Record<string, string> = {};
    for (const col of props.descriptor.columns) {
      const header = headerByColumnKey.get(col.key);
      const rawValue = header ? raw[header] : undefined;
      let value: unknown;
      try {
        value = col.parse ? col.parse(rawValue) : rawValue;
      } catch (err) {
        errors[col.key] = errorMessage(err);
        continue;
      }
      if (col.required && (value === undefined || value === null || value === '')) {
        errors[col.key] = `"${col.label}" مطلوب`;
        continue;
      }
      row[col.key] = value;
      const validationError = col.validate?.(value, row);
      if (validationError) errors[col.key] = validationError;
    }
    const status: ImportRowResult['status'] = Object.keys(errors).length ? 'error' : Object.keys(warnings).length ? 'warning' : 'ok';
    return { index, raw, row, errors, warnings, status };
  });
}

const okCount = computed(() => parsedRows.value.filter((r) => r.status === 'ok').length);
const warnCount = computed(() => parsedRows.value.filter((r) => r.status === 'warning').length);
const errorCount = computed(() => parsedRows.value.filter((r) => r.status === 'error').length);
const canImport = computed(() => okCount.value + warnCount.value > 0);

async function runImport() {
  stage.value = 'importing';
  progress.value = { done: 0, total: parsedRows.value.length };
  const importable = parsedRows.value.filter((r) => r.status !== 'error');
  try {
    const result = await props.descriptor.commit(importable, (done, total) => (progress.value = { done, total }));
    summary.value = result;
    stage.value = 'done';
    if (result.createdRows?.length) emit('imported', result.createdRows);
  } catch (err) {
    loadError.value = errorMessage(err);
    stage.value = 'validate';
  }
}

async function onDownloadErrors() {
  const failed = parsedRows.value.filter((r) => r.status === 'error').map((r) => ({ raw: r.raw, reason: Object.values(r.errors).join('؛ ') }));
  await downloadErrorFile(props.descriptor, failed);
}

function close() {
  emit('close');
}

const open = ref(true);
function onOpenChange(v: boolean) {
  if (!v) close();
}
</script>

<template>
  <AppModal :open="open" :title="`استيراد ${descriptor.title}`" size="lg" @update:open="onOpenChange">
    <div class="space-y-4">
      <p v-if="loadError" class="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger">{{ loadError }}</p>

      <!-- Upload stage -->
      <div v-if="stage === 'upload'" class="space-y-4">
        <div class="flex items-center justify-between rounded-lg border border-border bg-surface-hover/40 p-3 text-xs">
          <span>حمّل القالب الجاهز (رؤوس عربية، أمثلة، قوائم تحقق) لتعبئته.</span>
          <AppButton type="button" size="sm" :icon="Download" @click="onDownloadTemplate">تحميل القالب</AppButton>
        </div>
        <div
          class="flex flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed p-10 text-center text-xs transition-colors"
          :class="dragOver ? 'border-primary bg-primary/5' : 'border-border'"
          @dragover.prevent="dragOver = true"
          @dragleave.prevent="dragOver = false"
          @drop.prevent="onDrop"
        >
          <Upload class="size-6 text-text-secondary" />
          <p>اسحب ملف Excel أو CSV هنا، أو</p>
          <label class="cursor-pointer text-primary underline">
            اختر ملفاً
            <input type="file" accept=".xlsx,.csv" class="hidden" @change="onPick" />
          </label>
        </div>
      </div>

      <!-- Column mapping stage -->
      <div v-else-if="stage === 'map'" class="space-y-3">
        <p class="text-xs text-text-secondary">الملف: {{ fileName }} — {{ rawRows.length }} صف. راجع تعيين الأعمدة أدناه.</p>
        <div class="max-h-80 space-y-2 overflow-y-auto">
          <div v-for="h in headers" :key="h" class="flex items-center gap-2">
            <span class="w-40 shrink-0 truncate text-xs">{{ h }}</span>
            <AppSelect
              v-model="mapping[h]"
              class="flex-1"
              placeholder="تجاهل هذا العمود"
              :options="descriptor.columns.map((c) => ({ value: c.key, label: c.label + (c.required ? ' *' : '') }))"
            />
          </div>
        </div>
        <p v-if="unmappedRequired.length" class="flex items-center gap-1.5 text-xs text-warning">
          <TriangleAlert class="size-3.5" /> أعمدة مطلوبة غير معينة: {{ unmappedRequired.map((c) => c.label).join('، ') }}
        </p>
        <div class="flex justify-end gap-2">
          <AppButton type="button" @click="stage = 'upload'">رجوع</AppButton>
          <AppButton type="button" variant="primary" :disabled="unmappedRequired.length > 0" @click="confirmMapping">متابعة</AppButton>
        </div>
      </div>

      <!-- Validation table -->
      <div v-else-if="stage === 'validate'" class="space-y-3">
        <div class="flex flex-wrap items-center gap-3 text-xs">
          <span class="text-success">✓ صالح: {{ okCount }}</span>
          <span class="text-warning">⚠ تنبيه: {{ warnCount }}</span>
          <span class="text-danger">✗ خطأ: {{ errorCount }}</span>
          <AppButton v-if="errorCount" type="button" size="sm" class="ms-auto" @click="onDownloadErrors">تحميل ملف الأخطاء</AppButton>
        </div>
        <div class="max-h-80 overflow-auto rounded-lg border border-border">
          <table class="w-full text-xs">
            <thead class="sticky top-0 bg-surface">
              <tr>
                <th class="p-2 text-start">#</th>
                <th class="p-2 text-start">الحالة</th>
                <th v-for="c in descriptor.columns" :key="c.key" class="p-2 text-start">{{ c.label }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="r in parsedRows" :key="r.index" class="border-t border-border" :class="r.status === 'error' && 'bg-danger/5'">
                <td class="p-2 text-text-secondary">{{ r.index + 1 }}</td>
                <td class="p-2">
                  <span v-if="r.status === 'ok'" class="text-success">✓</span>
                  <span v-else-if="r.status === 'warning'" class="text-warning">⚠</span>
                  <span v-else class="text-danger">✗</span>
                </td>
                <td v-for="c in descriptor.columns" :key="c.key" class="p-2" :class="r.errors[c.key] && 'text-danger'">
                  {{ c.display ? c.display(r.row[c.key]) : (r.row[c.key] ?? '') }}
                  <span v-if="r.errors[c.key]" class="block text-caption">{{ r.errors[c.key] }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="flex justify-end gap-2">
          <AppButton type="button" @click="stage = 'map'">رجوع</AppButton>
          <AppButton type="button" variant="primary" :disabled="!canImport" @click="runImport">استيراد ({{ okCount + warnCount }})</AppButton>
        </div>
      </div>

      <!-- Importing -->
      <div v-else-if="stage === 'importing'" class="space-y-3 py-8 text-center text-xs">
        <p>جارٍ الاستيراد… {{ progress.done }} / {{ progress.total }}</p>
        <div class="mx-auto h-2 w-full max-w-sm overflow-hidden rounded-full bg-surface-hover">
          <div class="h-full bg-primary transition-all" :style="{ width: `${progress.total ? (progress.done / progress.total) * 100 : 0}%` }" />
        </div>
      </div>

      <!-- Done -->
      <div v-else-if="stage === 'done' && summary" class="space-y-3 py-4 text-center">
        <Check class="mx-auto size-8 text-success" />
        <p class="text-sm font-semibold">تم الاستيراد</p>
        <p class="text-xs text-text-secondary">أُنشئ {{ summary.created }} — تم تحديث {{ summary.updated }} — تم تخطي {{ summary.skipped }}</p>
        <AppButton v-if="summary.failed.length" type="button" size="sm" @click="onDownloadErrors">تحميل ملف الأخطاء ({{ summary.failed.length }})</AppButton>
        <div class="flex justify-center">
          <AppButton type="button" variant="primary" @click="close">إغلاق</AppButton>
        </div>
      </div>
    </div>
  </AppModal>
</template>
