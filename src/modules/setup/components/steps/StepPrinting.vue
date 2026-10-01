<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 10: default template + printer mode/width (Phase 11a/14's real settings, reused). */
import { onMounted, ref } from 'vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import InvoiceDocument from '@/modules/invoices/components/InvoiceDocument.vue';
import InvoiceThermal from '@/modules/invoices/components/InvoiceThermal.vue';
import { DEFAULT_A4_TEMPLATE } from '@/modules/invoices/helpers/invoiceTemplates';
import { getInvoicePrintData, type PrintData } from '@/modules/invoices/services/invoiceService';
import { getSettings, updateSettings } from '@/modules/settings/services/settingsService';
import type { WizardState } from '../../types';

const props = defineProps<{ state: WizardState }>();

const sample = ref<PrintData>();

// Serializes writes against the single `settings` row (each one locks it with `SELECT ... FOR
// UPDATE`): without this, a quick mode-then-width change — or clicking "التالي" right after a
// change — fires two overlapping `settings_update_settings` transactions and the second can hit
// the backend's lock-wait timeout ("قيد التعديل من جهاز آخر"). `commitCurrentStep` in
// SetupWizardPage awaits `pendingSave` before moving on, for the same reason.
let pendingSave: Promise<unknown> = Promise.resolve();

onMounted(async () => {
  const s = await getSettings();
  props.state.printing.printerMode = s.printer.mode;
  props.state.printing.thermalWidth = s.printer.thermalWidthMm;
  sample.value = await getInvoicePrintData('sample');
});

function save() {
  pendingSave = pendingSave
    .catch(() => {})
    .then(() => updateSettings({ printer: { mode: props.state.printing.printerMode, thermalWidthMm: props.state.printing.thermalWidth } as any }));
}

defineExpose({ flush: () => pendingSave });
</script>

<template>
  <div class="grid items-start gap-5 lg:grid-cols-[1fr_320px]">
    <div class="space-y-4">
      <AppCard title="وضع الطباعة">
        <SegmentedControl v-model="state.printing.printerMode" :options="[{ value: 'a4', label: 'A4' }, { value: 'thermal', label: 'حرارية' }]" @update:model-value="save" />
      </AppCard>
      <AppCard v-if="state.printing.printerMode === 'thermal'" title="عرض الورق الحراري">
        <AppSelect
          v-model.number="state.printing.thermalWidth"
          :options="[{ value: 58, label: '58 مم' }, { value: 80, label: '80 مم' }]"
          @update:model-value="save"
        />
      </AppCard>
      <p class="text-xs text-text-secondary">اختيار قالب الفاتورة الافتراضي متاح من الإعدادات → القوالب والطباعة.</p>
    </div>

    <AppCard title="معاينة" padding="sm">
      <div class="flex max-h-[60vh] justify-center overflow-auto rounded-lg bg-surface-hover p-4">
        <SkeletonBlock v-if="!sample" :lines="10" />
        <div v-else-if="state.printing.printerMode === 'thermal'" class="shadow-md">
          <InvoiceThermal :data="sample" :width="state.printing.thermalWidth" />
        </div>
        <div v-else class="origin-top scale-[0.4] bg-white shadow-md" style="margin-bottom: -60%">
          <InvoiceDocument :data="sample" layout="a4" :template-id="DEFAULT_A4_TEMPLATE" />
        </div>
      </div>
    </AppCard>
  </div>
</template>
