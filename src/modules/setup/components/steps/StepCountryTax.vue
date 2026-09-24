<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 3: country → base currency + VAT defaults, extra currencies. */
import { Plus, Trash2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import type { WizardState } from '../../types';

const props = defineProps<{ state: WizardState }>();

const COUNTRY_OPTIONS = [
  { value: 'SA', label: '🇸🇦 السعودية' },
  { value: 'EG', label: '🇪🇬 مصر' },
  { value: 'AE', label: '🇦🇪 الإمارات' },
];
const CURRENCY_BY_COUNTRY: Record<string, string> = { SA: 'SAR', EG: 'EGP', AE: 'AED' };

function onCountryChange() {
  props.state.countryTax.currency = CURRENCY_BY_COUNTRY[props.state.countryTax.country] ?? 'SAR';
}

function addCurrency() {
  props.state.countryTax.extraCurrencies.push({ code: '', rate: 1 });
}
function removeCurrency(i: number) {
  props.state.countryTax.extraCurrencies.splice(i, 1);
}
</script>

<template>
  <div class="space-y-4">
    <AppCard>
      <div class="grid gap-4 sm:grid-cols-2">
        <AppSelect v-model="state.countryTax.country" label="الدولة" :options="COUNTRY_OPTIONS" @update:model-value="onCountryChange" />
        <AppInput :model-value="state.countryTax.currency" label="العملة الأساسية" disabled hint="مأخوذة من الدولة — تُقفل بعد أول ترحيل" />
        <AppSwitch v-model="state.countryTax.vatRegistered" class="sm:col-span-2" label="مسجّل في ضريبة القيمة المضافة" description="يفعّل حقول الفاتورة الضريبية ويحسب 15% افتراضياً" />
        <AppSwitch
          v-model="state.countryTax.pricesIncludeTax"
          class="sm:col-span-2"
          label="الأسعار المعروضة شاملة الضريبة"
          description="الوضع الافتراضي للتجزئة في السعودية — يمكن تغييره لاحقاً من الإعدادات"
        />
      </div>
    </AppCard>

    <AppCard title="عملات إضافية (اختياري)">
      <p class="mb-3 text-xs text-text-secondary">لعملاء أو موردين يتعاملون بعملة غير العملة الأساسية.</p>
      <div v-for="(c, i) in state.countryTax.extraCurrencies" :key="i" class="mb-2 flex items-end gap-2">
        <AppInput v-model="c.code" class="w-28" label="رمز العملة" ltr placeholder="USD" />
        <AppInput v-model.number="c.rate" class="flex-1" type="number" min="0" step="0.0001" :label="`سعر الصرف (1 ${c.code || '?'} = ? ${state.countryTax.currency})`" />
        <button type="button" class="mb-2 rounded p-2 text-text-secondary hover:bg-surface-hover hover:text-danger" @click="removeCurrency(i)">
          <Trash2 class="size-4" />
        </button>
      </div>
      <AppButton type="button" size="sm" :icon="Plus" @click="addCurrency">إضافة عملة</AppButton>
    </AppCard>
  </div>
</template>
