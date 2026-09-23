<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { ImagePlus, Save, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { toNum } from '@/modules/core/helpers/numbers';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import { useSettingsStore } from '../controllers/useSettingsStore';
import { saveTax } from '../services/settingsService';
import type { Tax } from '../types';

const store = useSettingsStore();
const auth = useAuthStore();
const toast = useToast();
const canWrite = computed(() => auth.can('settings', 'write'));

const form = reactive({
  storeName: '',
  address: '',
  phone: '',
  commercialRegister: '',
  vatNumber: '',
  currency: 'SAR',
  invoiceNumberPrefix: 'INV-',
  receiptFooter: '',
  defaultTaxId: '',
  logo: undefined as string | undefined,
});
const errors = ref<Record<string, string>>({});
const saving = ref(false);
const loading = ref(true);
const fileInput = ref<HTMLInputElement>();

onMounted(async () => {
  await store.load(true);
  const s = store.settings!;
  Object.assign(form, {
    storeName: s.storeName,
    address: s.address ?? '',
    phone: s.phone ?? '',
    commercialRegister: s.commercialRegister ?? '',
    vatNumber: s.vatNumber ?? '',
    currency: s.currency,
    invoiceNumberPrefix: s.invoiceNumberPrefix,
    receiptFooter: s.receiptFooter ?? '',
    defaultTaxId: s.defaultTaxId ?? '',
    logo: s.logo,
  });
  loading.value = false;
});

function onLogo(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (!file) return;
  if (!file.type.startsWith('image/')) return toast.warning('اختر ملف صورة');
  if (file.size > 600 * 1024) return toast.warning('حجم الشعار كبير', 'الحد الأقصى 600 كيلوبايت');
  const reader = new FileReader();
  reader.onload = () => (form.logo = String(reader.result));
  reader.readAsDataURL(file);
}

async function save() {
  errors.value = {};
  if (!form.storeName.trim()) errors.value.storeName = 'اسم المتجر مطلوب';
  if (form.vatNumber && !/^3\d{13}3$/.test(form.vatNumber)) errors.value.vatNumber = '15 رقماً يبدأ وينتهي بالرقم 3';
  if (!form.invoiceNumberPrefix.trim()) errors.value.invoiceNumberPrefix = 'مطلوب';
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    await store.update({
      ...form,
      storeName: form.storeName.trim(),
      address: form.address.trim() || undefined,
      phone: form.phone.trim() || undefined,
      commercialRegister: form.commercialRegister.trim() || undefined,
      vatNumber: form.vatNumber.trim() || undefined,
      receiptFooter: form.receiptFooter.trim() || undefined,
      defaultTaxId: form.defaultTaxId || undefined,
    });
    toast.success('تم حفظ الإعدادات');
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

// --- taxes ---
const taxDraft = ref<Record<string, number | undefined>>({});
async function updateTax(tax: Tax, patch: Partial<Tax>) {
  try {
    await saveTax({ name: tax.name, rate: tax.rate, type: tax.type, isDefault: tax.isDefault, active: tax.active, ...patch }, tax.id);
    await store.reloadTaxes();
    toast.success('تم تحديث الضريبة', tax.name);
  } catch (err) {
    toast.error(err);
  }
}
function commitRate(tax: Tax) {
  const rate = toNum(taxDraft.value[tax.id]);
  if (rate === undefined || rate === tax.rate) return;
  updateTax(tax, { rate });
}

const outputTaxes = computed(() => store.taxes.filter((t) => t.type === 'OUTPUT').map((t) => ({ value: t.id, label: `${t.name} (${t.rate}%)` })));
</script>

<template>
  <div>
    <PageHeader title="الإعدادات" subtitle="بيانات المتجر التي تظهر على الفواتير، والضرائب" />
    <SettingsTabs />

    <SkeletonBlock v-if="loading" :lines="8" height="h-9" />
    <form v-else class="grid items-start gap-5 xl:grid-cols-[1fr_360px]" novalidate @submit.prevent="save">
      <div class="space-y-5">
        <AppCard title="بيانات المتجر">
          <div class="grid gap-4 sm:grid-cols-2">
            <AppInput v-model="form.storeName" class="sm:col-span-2" label="اسم المتجر" required :disabled="!canWrite" :error="errors.storeName" />
            <AppInput v-model="form.address" class="sm:col-span-2" label="العنوان" :disabled="!canWrite" />
            <AppInput v-model="form.phone" label="الهاتف" ltr :disabled="!canWrite" />
            <AppInput v-model="form.commercialRegister" label="السجل التجاري" ltr :disabled="!canWrite" />
            <AppInput v-model="form.vatNumber" label="الرقم الضريبي" ltr :disabled="!canWrite" :error="errors.vatNumber" hint="يظهر على الفاتورة وفي رمز QR" />
            <AppSelect
              v-model="form.currency"
              label="العملة"
              :disabled="!canWrite"
              :options="[
                { value: 'SAR', label: 'ريال سعودي (SAR)' },
                { value: 'AED', label: 'درهم إماراتي (AED)' },
                { value: 'KWD', label: 'دينار كويتي (KWD)' },
                { value: 'USD', label: 'دولار أمريكي (USD)' },
              ]"
            />
          </div>
        </AppCard>

        <AppCard title="الفواتير">
          <div class="grid gap-4 sm:grid-cols-2">
            <AppInput v-model="form.invoiceNumberPrefix" label="بادئة رقم الفاتورة" ltr :disabled="!canWrite" :error="errors.invoiceNumberPrefix" hint="مثال: INV- تنتج INV-000123" />
            <AppSelect v-model="form.defaultTaxId" label="ضريبة المبيعات الافتراضية" :disabled="!canWrite" :options="outputTaxes" />
            <AppInput v-model="form.receiptFooter" class="sm:col-span-2" label="نص أسفل الفاتورة" :disabled="!canWrite" placeholder="مثال: الاستبدال خلال 7 أيام" />
          </div>
        </AppCard>

        <AppCard title="الضرائب" padding="none">
          <table class="w-full text-body">
            <thead class="text-xs text-text-secondary">
              <tr class="border-b border-border">
                <th class="px-4 py-2 text-start font-medium">الضريبة</th>
                <th class="px-2 py-2 text-start font-medium">النوع</th>
                <th class="px-2 py-2 text-start font-medium">النسبة</th>
                <th class="px-4 py-2 text-start font-medium">نشطة</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="t in store.taxes" :key="t.id" class="border-b border-border last:border-0">
                <td class="px-4 py-2">
                  {{ t.name }}
                  <span v-if="t.isDefault" class="ms-1.5 rounded-full bg-primary/10 px-1.5 text-tiny text-primary">افتراضية</span>
                </td>
                <td class="px-2 py-2 text-text-secondary">{{ t.type === 'OUTPUT' ? 'مخرجات (مبيعات)' : 'مدخلات (مشتريات)' }}</td>
                <td class="px-2 py-2">
                  <input
                    :value="taxDraft[t.id] ?? t.rate"
                    type="number"
                    min="0"
                    max="100"
                    :disabled="!canWrite"
                    class="control h-8 w-20"
                    @input="taxDraft[t.id] = toNum(($event.target as HTMLInputElement).value)"
                    @change="commitRate(t)"
                  />
                  %
                </td>
                <td class="px-4 py-2"><AppSwitch :model-value="t.active" :disabled="!canWrite || t.isDefault" @update:model-value="(v) => updateTax(t, { active: v })" /></td>
              </tr>
            </tbody>
          </table>
        </AppCard>
      </div>

      <div class="space-y-5 xl:sticky xl:top-0">
        <AppCard title="الشعار">
          <div class="flex items-center gap-4">
            <div class="flex size-20 shrink-0 items-center justify-center overflow-hidden rounded-lg border border-dashed border-border bg-background">
              <img v-if="form.logo" :src="form.logo" alt="شعار المتجر" class="size-full object-contain" />
              <ImagePlus v-else class="size-6 text-text-secondary" />
            </div>
            <div class="space-y-2">
              <input ref="fileInput" type="file" accept="image/*" class="hidden" @change="onLogo" />
              <AppButton size="sm" :icon="ImagePlus" :disabled="!canWrite" @click="fileInput?.click()">{{ form.logo ? 'تغيير' : 'رفع شعار' }}</AppButton>
              <AppButton v-if="form.logo" size="sm" variant="ghost" :icon="Trash" :disabled="!canWrite" @click="form.logo = undefined">إزالة</AppButton>
              <p class="text-tiny text-text-secondary">PNG أو JPG، حتى 600 كيلوبايت. يُحفظ محلياً.</p>
            </div>
          </div>
        </AppCard>
        <AppButton v-if="canWrite" type="submit" variant="primary" block :icon="Save" :loading="saving">حفظ الإعدادات</AppButton>
      </div>
    </form>
  </div>
</template>
