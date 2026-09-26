<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { isTauri } from '@tauri-apps/api/core';
import { FileText, Printer, ReceiptText, Tag } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import * as printService from '@/modules/core/services/printService';
import type { PrinterInfo } from '@/modules/core/services/printService';
import InvoiceA4 from '@/modules/invoices/components/InvoiceA4.vue';
import InvoiceThermal from '@/modules/invoices/components/InvoiceThermal.vue';
import { getInvoicePrintData, type PrintData } from '@/modules/invoices/services/invoiceService';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import { useSettingsStore } from '../controllers/useSettingsStore';
import type { PrinterConnectionType, PrinterMode, ThermalPrinterSettings, ThermalWidth } from '../types';

const store = useSettingsStore();
const auth = useAuthStore();
const router = useRouter();
const toast = useToast();
const canWrite = computed(() => auth.can('settings', 'write'));
const inTauri = isTauri();

const mode = ref<PrinterMode>(store.settings?.printer.mode ?? 'a4');
const width = ref<ThermalWidth>(store.settings?.printer.thermalWidthMm ?? 80);
const sample = ref<PrintData>();

// Phase 14 (docs/v2/12-documents-pdf-excel.md §5): native receipt-printer
// transport settings, additive to the mock-phase mode/width above.
const connection = ref<PrinterConnectionType>(store.settings?.printer.thermal?.connection ?? 'windows');
const printerName = ref(store.settings?.printer.thermal?.printerName ?? '');
const host = ref(store.settings?.printer.thermal?.host ?? '');
const dpi = ref(store.settings?.printer.thermal?.dpi ?? 203);
const cut = ref(store.settings?.printer.thermal?.cut ?? true);
const openDrawer = ref(store.settings?.printer.thermal?.openDrawer ?? false);
const copies = ref(store.settings?.printer.thermal?.copies ?? 1);
const a4PrinterName = ref(store.settings?.printer.a4PrinterName ?? '');
const labelPrinterName = ref(store.settings?.printer.labelPrinterName ?? '');

const availablePrinters = ref<PrinterInfo[]>([]);
const loadingPrinters = ref(false);
const testing = ref(false);

onMounted(async () => {
  sample.value = await getInvoicePrintData('sample');
  await loadPrinters();
});

async function loadPrinters() {
  if (!inTauri) return;
  loadingPrinters.value = true;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    availablePrinters.value = await invoke<PrinterInfo[]>('list_printers');
    // Default the A4 field to the OS default printer the first time the page loads with nothing saved yet.
    if (!a4PrinterName.value) {
      const def = availablePrinters.value.find((p) => p.is_default);
      if (def) a4PrinterName.value = def.name;
    }
  } catch (err) {
    toast.error(err, 'تعذر جلب قائمة الطابعات');
  } finally {
    loadingPrinters.value = false;
  }
}

// Auto-save: printing preferences are tiny and changing them is never destructive.
watch([mode, width], async () => {
  if (!canWrite.value) return;
  try {
    await store.update({ printer: { mode: mode.value, thermalWidthMm: width.value } });
    toast.success('تم حفظ إعدادات الطباعة', mode.value === 'a4' ? 'A4' : `حراري ${width.value}mm`);
  } catch (err) {
    toast.error(err);
  }
});

const thermalConfig = computed<ThermalPrinterSettings>(() => ({
  printerName: printerName.value || undefined,
  connection: connection.value,
  host: host.value || undefined,
  dpi: dpi.value,
  cut: cut.value,
  openDrawer: openDrawer.value,
  copies: copies.value,
}));

watch(
  [connection, printerName, host, dpi, cut, openDrawer, copies, a4PrinterName, labelPrinterName],
  async () => {
    if (!canWrite.value) return;
    try {
      await store.update({
        printer: {
          mode: mode.value,
          thermalWidthMm: width.value,
          thermal: thermalConfig.value,
          a4PrinterName: a4PrinterName.value || undefined,
          labelPrinterName: labelPrinterName.value || undefined,
        },
      });
    } catch (err) {
      toast.error(err);
    }
  },
  { deep: true },
);

function testPrint() {
  router.push({ name: 'invoice-print', params: { id: 'sample' }, query: { mode: mode.value, width: String(width.value) } });
}

async function testThermalPrint() {
  testing.value = true;
  try {
    const result = await printService.testPrint(thermalConfig.value, width.value);
    if (result.ok) toast.success('تم إرسال إيصال الاختبار للطابعة');
    else toast.error(result.error ?? 'تعذرت الطباعة', 'فشل اختبار الطباعة');
  } finally {
    testing.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="الإعدادات" subtitle="نوع الطابعة ومقاس الورق للفواتير والإيصالات" />
    <SettingsTabs />

    <div class="grid items-start gap-5 xl:grid-cols-[1fr_420px]">
      <div class="space-y-5">
        <AppCard title="نوع الطباعة الافتراضي">
          <div class="grid gap-3 sm:grid-cols-2">
            <button
              v-for="opt in [
                { value: 'a4', title: 'A4 — فاتورة كاملة', desc: 'طابعات المكتب العادية. فاتورة ضريبية بجدول أصناف كامل.', icon: FileText },
                { value: 'thermal', title: 'حراري — إيصال', desc: 'طابعات الإيصالات في نقطة البيع (58 أو 80 ملم).', icon: ReceiptText },
              ] as const"
              :key="opt.value"
              type="button"
              :disabled="!canWrite"
              class="flex items-start gap-3 rounded-xl border p-4 text-start transition-colors disabled:cursor-not-allowed"
              :class="mode === opt.value ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
              @click="mode = opt.value"
            >
              <component :is="opt.icon" class="mt-0.5 size-5 shrink-0" :class="mode === opt.value ? 'text-primary' : 'text-text-secondary'" />
              <span>
                <span class="block text-body font-medium">{{ opt.title }}</span>
                <span class="mt-0.5 block text-xs leading-5 text-text-secondary">{{ opt.desc }}</span>
              </span>
            </button>
          </div>
          <div v-if="mode === 'thermal'" class="mt-5">
            <span class="field-label">عرض ورق الطابعة الحرارية</span>
            <SegmentedControl
              v-model="width"
              :options="[
                { value: 80, label: '80 ملم (الأكثر شيوعاً)' },
                { value: 58, label: '58 ملم' },
              ]"
            />
          </div>
        </AppCard>

        <AppCard v-if="mode === 'thermal'" title="طابعة الإيصالات (اتصال مباشر)">
          <p v-if="!inTauri" class="mb-4 text-body leading-6 text-text-secondary">
            الاتصال المباشر بالطابعة الحرارية متاح في نسخة سطح المكتب فقط. في المتصفح، تُستخدم نافذة الطباعة العادية.
          </p>
          <div class="space-y-4">
            <div>
              <span class="field-label">نوع الاتصال</span>
              <SegmentedControl
                v-model="connection"
                :options="[
                  { value: 'windows', label: 'طابعة النظام (USB)' },
                  { value: 'network', label: 'طابعة شبكة (IP)' },
                ]"
              />
            </div>

            <AppSelect
              v-if="connection === 'windows'"
              v-model="printerName"
              label="اسم الطابعة"
              placeholder="اختر طابعة"
              :options="availablePrinters.map((p) => ({ value: p.name, label: p.is_default ? `${p.name} (الافتراضية)` : p.name }))"
              :disabled="!canWrite || loadingPrinters"
              :hint="!inTauri ? 'يتطلب نسخة سطح المكتب' : loadingPrinters ? 'جارٍ التحميل...' : undefined"
            />
            <AppInput v-else v-model="host" label="عنوان IP للطابعة" placeholder="192.168.1.50" ltr :disabled="!canWrite" hint="المنفذ 9100 دائماً" />

            <div class="grid gap-4 sm:grid-cols-2">
              <AppInput v-model="dpi" type="number" label="الدقة (DPI)" :min="150" :max="300" :disabled="!canWrite" />
              <AppInput v-model="copies" type="number" label="عدد النسخ" :min="1" :max="5" :disabled="!canWrite" />
            </div>

            <AppSwitch v-model="cut" label="قص الورق تلقائياً" description="يرسل أمر القص بعد كل إيصال" :disabled="!canWrite" />
            <AppSwitch v-model="openDrawer" label="فتح درج النقدية" description="عند البيع نقداً فقط" :disabled="!canWrite" />

            <AppButton variant="primary" :icon="Printer" :loading="testing" :disabled="!inTauri" @click="testThermalPrint">اختبار الطباعة</AppButton>
          </div>
        </AppCard>

        <AppCard title="طابعات أخرى">
          <div class="space-y-4">
            <AppSelect
              v-model="a4PrinterName"
              label="طابعة A4"
              placeholder="طابعة النظام الافتراضية"
              :options="availablePrinters.map((p) => ({ value: p.name, label: p.is_default ? `${p.name} (الافتراضية)` : p.name }))"
              :disabled="!canWrite"
              hint="معلوماتية فقط — تُستخدم طابعة النظام الافتراضية دائماً لفواتير A4"
            />
            <AppInput v-model="labelPrinterName" label="طابعة الملصقات" placeholder="اسم طابعة الملصقات (اختياري)" :disabled="!canWrite" hint="حقل مبدئي فقط — طباعة الملصقات نفسها قادمة في مرحلة لاحقة">
              <template #prefix><Tag class="size-4" /></template>
            </AppInput>
          </div>
        </AppCard>

        <AppCard title="معاينة الطباعة (المتصفح)">
          <p class="mb-4 text-body leading-6 text-text-secondary">
            يفتح نموذج فاتورة تجريبي بالإعدادات الحالية في نافذة معاينة الطباعة العادية. مستقل عن الاتصال المباشر أعلاه — مفيد كبديل احتياطي.
          </p>
          <AppButton variant="secondary" :icon="Printer" @click="testPrint">فتح نافذة المعاينة</AppButton>
        </AppCard>
      </div>

      <AppCard title="معاينة" padding="sm">
        <div class="flex max-h-[70vh] justify-center overflow-auto rounded-lg bg-surface-hover p-4">
          <SkeletonBlock v-if="!sample" :lines="10" />
          <div v-else-if="mode === 'thermal'" class="shadow-md">
            <InvoiceThermal :data="sample" :width="width" />
          </div>
          <div v-else class="origin-top scale-[0.48] bg-white p-[12mm] shadow-md" style="margin-bottom: -52%">
            <InvoiceA4 :data="sample" />
          </div>
        </div>
      </AppCard>
    </div>
  </div>
</template>
