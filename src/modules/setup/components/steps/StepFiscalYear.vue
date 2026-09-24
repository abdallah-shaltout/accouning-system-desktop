<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 4: fiscal year start + go-live (opening-balance) date. */
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import type { WizardState } from '../../types';

defineProps<{ state: WizardState }>();

const MONTHS = [
  { value: 1, label: 'يناير' }, { value: 2, label: 'فبراير' }, { value: 3, label: 'مارس' }, { value: 4, label: 'أبريل' },
  { value: 5, label: 'مايو' }, { value: 6, label: 'يونيو' }, { value: 7, label: 'يوليو' }, { value: 8, label: 'أغسطس' },
  { value: 9, label: 'سبتمبر' }, { value: 10, label: 'أكتوبر' }, { value: 11, label: 'نوفمبر' }, { value: 12, label: 'ديسمبر' },
];
</script>

<template>
  <div class="space-y-4">
    <AppCard title="السنة المالية">
      <div class="grid gap-4 sm:grid-cols-2">
        <AppSelect v-model.number="state.fiscalYear.startMonth" label="شهر البداية" :options="MONTHS" />
        <AppInput v-model.number="state.fiscalYear.startDay" type="number" min="1" max="28" label="يوم البداية" />
      </div>
    </AppCard>
    <AppCard title="تاريخ البدء (Go-live)">
      <AppInput v-model="state.fiscalYear.goLiveDate" type="date" label="تاريخ بدء العمل بالنظام" required />
      <p class="mt-2 text-xs text-text-secondary">هذا هو تاريخ الأرصدة الافتتاحية — يُقفل بعد ترحيلها في الخطوة 8.</p>
    </AppCard>
  </div>
</template>
