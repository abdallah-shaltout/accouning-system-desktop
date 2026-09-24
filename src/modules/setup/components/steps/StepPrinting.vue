<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 10: default template + printer mode/width (Phase 11a/14's real settings, reused). */
import { onMounted } from 'vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { getSettings, updateSettings } from '@/modules/settings/services/settingsService';
import type { WizardState } from '../../types';

const props = defineProps<{ state: WizardState }>();

onMounted(async () => {
  const s = await getSettings();
  props.state.printing.printerMode = s.printer.mode;
  props.state.printing.thermalWidth = s.printer.thermalWidthMm;
});

async function save() {
  await updateSettings({ printer: { mode: props.state.printing.printerMode, thermalWidthMm: props.state.printing.thermalWidth } as any });
}
</script>

<template>
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
    <p class="text-xs text-text-secondary">اختيار قالب الفاتورة الافتراضي ومعاينة الطابعة متاحان من الإعدادات → القوالب والطباعة.</p>
  </div>
</template>
