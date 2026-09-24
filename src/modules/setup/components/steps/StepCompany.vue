<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 2: company details. Writes straight into StoreSettings so every printed document picks it up immediately. */
import { onMounted, ref, watch } from 'vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppPhoneInput from '@/modules/core/components/ui/AppPhoneInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { getSettings, updateSettings } from '@/modules/settings/services/settingsService';
import type { WizardState } from '../../types';

const props = defineProps<{ state: WizardState }>();
const vatError = ref('');

watch(
  () => props.state.company.vatNumber,
  (v) => {
    vatError.value = v && !/^3\d{13}3$/.test(v) ? '15 رقماً يبدأ وينتهي بالرقم 3' : '';
  },
);

// Load whatever's already in settings (e.g. a previous partial run) as the starting point.
onMounted(async () => {
  const s = await getSettings();
  if (s.storeName && s.storeName !== 'شركتي') props.state.company.nameAr = s.storeName;
  props.state.company.vatNumber = s.vatNumber ?? props.state.company.vatNumber;
  props.state.company.phone = s.phone ?? props.state.company.phone;
});

// Auto-save on every change so leaving the wizard mid-step doesn't lose company info — cheap since `updateSettings` is a mutate() call, not a network round-trip.
let saveTimer: ReturnType<typeof setTimeout> | undefined;
watch(
  () => JSON.stringify(props.state.company),
  () => {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      void updateSettings({
        storeName: props.state.company.nameAr || 'شركتي',
        vatNumber: props.state.company.vatNumber || undefined,
        phone: props.state.company.phone || undefined,
        address: [props.state.company.nationalAddress.city, props.state.company.nationalAddress.district, props.state.company.nationalAddress.street].filter(Boolean).join('، ') || undefined,
      });
    }, 400);
  },
  { deep: true },
);
</script>

<template>
  <div class="space-y-4">
    <AppCard>
      <div class="grid gap-4 sm:grid-cols-2">
        <div class="sm:col-span-2">
          <span class="field-label">النوع</span>
          <SegmentedControl v-model="state.company.type" :options="[{ value: 'company', label: 'منشأة' }, { value: 'individual', label: 'فرد' }]" />
        </div>
        <AppInput v-model="state.company.nameAr" label="اسم المنشأة (عربي)" required class="sm:col-span-2" />
        <AppInput v-model="state.company.nameEn" label="الاسم (إنجليزي، اختياري)" ltr />
        <AppInput v-model="state.company.vatNumber" label="الرقم الضريبي" ltr placeholder="3xxxxxxxxxxxxx3" :error="vatError" />
        <AppInput v-model="state.company.crNumber" label="رقم السجل التجاري" ltr />
        <AppPhoneInput v-model="state.company.phone" label="الهاتف" />
        <AppInput v-model="state.company.email" label="البريد الإلكتروني" type="email" ltr />
      </div>
    </AppCard>

    <AppCard title="العنوان الوطني">
      <div class="grid gap-4 sm:grid-cols-3">
        <AppInput v-model="state.company.nationalAddress.city" label="المدينة" />
        <AppInput v-model="state.company.nationalAddress.district" label="الحي" />
        <AppInput v-model="state.company.nationalAddress.street" label="الشارع" />
        <AppInput v-model="state.company.nationalAddress.buildingNo" label="رقم المبنى" ltr />
        <AppInput v-model="state.company.nationalAddress.postalCode" label="الرمز البريدي" ltr />
      </div>
    </AppCard>
  </div>
</template>
