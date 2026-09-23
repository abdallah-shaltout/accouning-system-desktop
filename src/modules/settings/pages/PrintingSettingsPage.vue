<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { FileText, Printer, ReceiptText } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import InvoiceA4 from '@/modules/invoices/components/InvoiceA4.vue';
import InvoiceThermal from '@/modules/invoices/components/InvoiceThermal.vue';
import { getInvoicePrintData, type PrintData } from '@/modules/invoices/services/invoiceService';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import { useSettingsStore } from '../controllers/useSettingsStore';
import type { PrinterMode, ThermalWidth } from '../types';

const store = useSettingsStore();
const auth = useAuthStore();
const router = useRouter();
const toast = useToast();
const canWrite = computed(() => auth.can('settings', 'write'));

const mode = ref<PrinterMode>(store.settings?.printer.mode ?? 'a4');
const width = ref<ThermalWidth>(store.settings?.printer.thermalWidthMm ?? 80);
const sample = ref<PrintData>();

onMounted(async () => (sample.value = await getInvoicePrintData('sample')));

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

function testPrint() {
  router.push({ path: '/print/invoices/sample', query: { mode: mode.value, width: String(width.value), back: '/settings/printing' } });
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

        <AppCard title="اختبار الطباعة">
          <p class="mb-4 text-body leading-6 text-text-secondary">
            يفتح نموذج فاتورة تجريبي بالإعدادات الحالية في نافذة معاينة الطباعة. لا يتم الاتصال بأي جهاز في هذه المرحلة — اختر الطابعة من نافذة الطباعة في النظام.
          </p>
          <AppButton variant="primary" :icon="Printer" @click="testPrint">اختبار الطباعة</AppButton>
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
