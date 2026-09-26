<script setup lang="ts">
/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §2): Settings → Currencies. The base
 * currency itself is set/locked on the general settings page — this page manages OTHER enabled
 * currencies and their exchange-rate history. Rate entry accepts either direction ("1 USD = 3.75
 * SAR" or "1 SAR = 0.2667 USD") — `saveExchangeRate` inverts the second form before saving.
 */
import { computed, onMounted, reactive, ref } from 'vue';
import { Plus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useSettingsStore } from '../controllers/useSettingsStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import { createCurrency, getCurrencies, getExchangeRates, saveExchangeRate, updateCurrency } from '../services/branchesService';
import type { Currency } from '../types';

const auth = useAuthStore();
const toast = useToast();
const settingsStore = useSettingsStore();
const canWrite = computed(() => auth.can('settings', 'write'));

const loading = ref(true);
const currencies = ref<Currency[]>([]);
const rates = ref<Record<string, { date: string; rate: number }[]>>({});

async function reload() {
  currencies.value = await getCurrencies();
  const allRates = await getExchangeRates();
  rates.value = {};
  for (const r of allRates) (rates.value[r.currency] ??= []).push({ date: r.date, rate: r.rate });
  for (const code of Object.keys(rates.value)) rates.value[code].sort((a, b) => b.date.localeCompare(a.date));
}

onMounted(async () => {
  await settingsStore.load();
  await reload();
  loading.value = false;
});

function latestRate(code: string): number | undefined {
  return rates.value[code]?.[0]?.rate;
}

// --- Add currency ---
const currencyFormOpen = ref(false);
const savingCurrency = ref(false);
const currencyErrors = ref<Record<string, string>>({});
const currencyForm = reactive({ code: '', nameAr: '', symbol: '', decimals: 2, fixed: false, fixedRate: undefined as number | undefined });

function openAddCurrency() {
  Object.assign(currencyForm, { code: '', nameAr: '', symbol: '', decimals: 2, fixed: false, fixedRate: undefined });
  currencyErrors.value = {};
  currencyFormOpen.value = true;
}

async function saveCurrency() {
  currencyErrors.value = {};
  if (!currencyForm.code.trim()) currencyErrors.value.code = 'رمز العملة مطلوب (مثال: USD)';
  if (!currencyForm.nameAr.trim()) currencyErrors.value.nameAr = 'الاسم بالعربية مطلوب';
  if (Object.keys(currencyErrors.value).length) return;
  savingCurrency.value = true;
  try {
    await createCurrency({
      code: currencyForm.code.trim().toUpperCase(),
      nameAr: currencyForm.nameAr.trim(),
      symbol: currencyForm.symbol.trim(),
      decimals: currencyForm.decimals,
      active: true,
      fixed: currencyForm.fixed,
      fixedRate: currencyForm.fixed ? currencyForm.fixedRate : undefined,
    });
    await reload();
    toast.success('تمت إضافة العملة');
    currencyFormOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    savingCurrency.value = false;
  }
}

async function toggleActive(c: Currency) {
  try {
    await updateCurrency(c.code, { active: !c.active });
    await reload();
  } catch (err) {
    toast.error(err);
  }
}

// --- Rate entry ---
const rateFormOpen = ref(false);
const rateCurrency = ref('');
const savingRate = ref(false);
const rateDirection = ref<'direct' | 'inverse'>('direct');
const rateForm = reactive({ date: new Date().toISOString().slice(0, 10), rate: undefined as number | undefined, inverseRate: undefined as number | undefined });

function openAddRate(code: string) {
  rateCurrency.value = code;
  rateDirection.value = 'direct';
  Object.assign(rateForm, { date: new Date().toISOString().slice(0, 10), rate: latestRate(code), inverseRate: undefined });
  rateFormOpen.value = true;
}

async function saveRate() {
  if (!rateForm.date) return;
  savingRate.value = true;
  try {
    await saveExchangeRate({
      currency: rateCurrency.value,
      date: rateForm.date,
      rate: rateDirection.value === 'direct' ? rateForm.rate : undefined,
      inverseRate: rateDirection.value === 'inverse' ? rateForm.inverseRate : undefined,
    });
    await reload();
    toast.success('تم حفظ سعر الصرف');
    rateFormOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    savingRate.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="العملات" :subtitle="`العملة الأساسية: ${settingsStore.currency} (تُعدّل من تبويب «عام»)`" />
    <SettingsTabs />

    <SkeletonBlock v-if="loading" :lines="4" height="h-12" />
    <div v-else class="space-y-5">
      <AppCard title="العملات المفعّلة" padding="none">
        <template #actions>
          <AppButton v-if="canWrite" size="sm" :icon="Plus" @click="openAddCurrency">إضافة عملة</AppButton>
        </template>
        <EmptyState v-if="!currencies.length" title="لا توجد عملات إضافية" description="أضف عملة مثل الدولار لبيع عملاء أجانب أو فتح صندوق/حساب بنكي بعملة أجنبية" />
        <table v-else class="w-full text-body">
          <thead class="text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2 text-start font-medium">العملة</th>
              <th class="px-2 py-2 text-start font-medium">الرمز</th>
              <th class="px-2 py-2 text-start font-medium">آخر سعر صرف</th>
              <th class="px-2 py-2 text-start font-medium">نشطة</th>
              <th class="px-4 py-2"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="c in currencies" :key="c.code" class="border-b border-border last:border-0">
              <td class="px-4 py-2 font-medium">{{ c.nameAr }} <span class="text-tiny text-text-secondary">({{ c.symbol }})</span></td>
              <td class="px-2 py-2"><span class="num text-text-secondary">{{ c.code }}</span></td>
              <td class="px-2 py-2">
                <span v-if="c.fixed" class="num">{{ c.fixedRate }} (ثابت)</span>
                <span v-else-if="latestRate(c.code)" class="num">{{ latestRate(c.code) }}</span>
                <span v-else class="text-text-secondary">—</span>
              </td>
              <td class="px-2 py-2"><AppSwitch :model-value="c.active" :disabled="!canWrite" @update:model-value="() => toggleActive(c)" /></td>
              <td class="px-4 py-2 text-end">
                <AppButton v-if="!c.fixed && canWrite" size="sm" variant="ghost" @click="openAddRate(c.code)">سعر جديد</AppButton>
              </td>
            </tr>
          </tbody>
        </table>
      </AppCard>

      <AppCard v-for="c in currencies.filter((x) => !x.fixed && rates[x.code]?.length)" :key="`rates-${c.code}`" :title="`سجل أسعار الصرف — ${c.nameAr}`" padding="none">
        <table class="w-full text-body">
          <thead class="text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2 text-start font-medium">التاريخ</th>
              <th class="px-2 py-2 text-start font-medium">السعر (1 {{ c.code }} = ؟ {{ settingsStore.currency }})</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in rates[c.code]" :key="r.date" class="border-b border-border last:border-0">
              <td class="px-4 py-2 num">{{ r.date }}</td>
              <td class="px-2 py-2 num">{{ r.rate }}</td>
            </tr>
          </tbody>
        </table>
      </AppCard>
    </div>

    <AppModal v-model:open="currencyFormOpen" title="عملة جديدة" :persistent="savingCurrency">
      <form class="space-y-4" novalidate @submit.prevent="saveCurrency">
        <AppInput v-model="currencyForm.code" label="الرمز (ISO)" required ltr placeholder="USD" :error="currencyErrors.code" />
        <AppInput v-model="currencyForm.nameAr" label="الاسم بالعربية" required :error="currencyErrors.nameAr" />
        <AppInput v-model="currencyForm.symbol" label="الرمز المختصر" ltr placeholder="$" />
        <AppInput v-model="currencyForm.decimals" type="number" min="0" max="4" label="عدد الخانات العشرية" />
        <AppSwitch v-model="currencyForm.fixed" label="سعر ثابت (عملة مربوطة)" description="مثل الريال السعودي أو الدرهم مقابل الدولار — لا حاجة لجدول أسعار" />
        <AppInput v-if="currencyForm.fixed" v-model="currencyForm.fixedRate" type="number" step="0.0001" label="السعر الثابت" />
      </form>
      <template #footer>
        <AppButton :disabled="savingCurrency" @click="currencyFormOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="savingCurrency" @click="saveCurrency">حفظ</AppButton>
      </template>
    </AppModal>

    <AppModal v-model:open="rateFormOpen" :title="`سعر صرف جديد — ${rateCurrency}`" :persistent="savingRate">
      <form class="space-y-4" novalidate @submit.prevent="saveRate">
        <AppDatePicker v-model="rateForm.date" label="التاريخ" required />
        <div class="flex gap-2 text-tiny">
          <button type="button" class="rounded-md border px-2 py-1" :class="rateDirection === 'direct' ? 'border-primary bg-primary/10 text-primary' : 'border-border text-text-secondary'" @click="rateDirection = 'direct'">
            1 {{ rateCurrency }} = ؟ {{ settingsStore.currency }}
          </button>
          <button type="button" class="rounded-md border px-2 py-1" :class="rateDirection === 'inverse' ? 'border-primary bg-primary/10 text-primary' : 'border-border text-text-secondary'" @click="rateDirection = 'inverse'">
            1 {{ settingsStore.currency }} = ؟ {{ rateCurrency }}
          </button>
        </div>
        <AppInput v-if="rateDirection === 'direct'" v-model="rateForm.rate" type="number" step="0.0001" :label="`1 ${rateCurrency} = ${settingsStore.currency}`" />
        <AppInput v-else v-model="rateForm.inverseRate" type="number" step="0.0001" :label="`1 ${settingsStore.currency} = ${rateCurrency}`" />
      </form>
      <template #footer>
        <AppButton :disabled="savingRate" @click="rateFormOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="savingRate" @click="saveRate">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
