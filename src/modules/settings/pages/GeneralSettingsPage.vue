<script setup lang="ts">
// v2 §5 (docs/v2/14-platform.md §5 — "company settings" branding row): logo/stamp/signature
// uploads. These stay on the same lightweight base64-data-URL pattern the logo field already used
// rather than switching to `AttachmentField` (its async IndexedDB blob store) — `pdfService.ts`'s
// `companyBlock()` reads `company.logo`/`stamp`/`signature` synchronously by value when building a
// Typst document payload, so the image has to be an inline string, not a blob reference.
import { computed, onMounted, reactive, ref } from 'vue';
import { Save } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import SettingsPage from '@/modules/core/components/layouts/SettingsPage.vue';
import AddressFields from '@/modules/core/components/blocks/AddressFields.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import ImageUploadField from '../components/ImageUploadField.vue';
import SettingsTabs from '../components/SettingsTabs.vue';
import { useSettingsStore } from '../controllers/useSettingsStore';
import { isBaseCurrencyLocked } from '../services/branchesService';
import { countryProfile, CURRENCY_OPTIONS, DEFAULT_COUNTRY } from '@/modules/core/helpers/countryProfiles';
import { formatAddress } from '@/modules/core/helpers/format';
import type { Address } from '@/modules/core/types/address';

const store = useSettingsStore();
const auth = useAuthStore();
const toast = useToast();
const canWrite = computed(() => auth.can('settings', 'write'));
const currencyLocked = ref(false);
// v2 doc 18.D: the VAT-number field's label/pattern/hint follow the company's own country, not a
// hard-coded Saudi pattern — falls back to the default country's profile until settings load.
const activeProfile = computed(() => countryProfile(store.settings?.country ?? DEFAULT_COUNTRY));

const form = reactive({
  storeName: '',
  address: '' as string,
  nationalAddress: { country: DEFAULT_COUNTRY } as Address,
  phone: '',
  commercialRegister: '',
  vatNumber: '',
  currency: 'SAR',
  invoiceNumberPrefix: 'INV-',
  receiptFooter: '',
  defaultTaxId: '',
  /** v2 (docs/v2/06-sales-and-pos.md §3, README decision 4): default true — typical B2C shelf pricing; overwritten by `store.load()` with the company's actual saved value. */
  pricesIncludeTax: true,
  logo: undefined as string | undefined,
  stamp: undefined as string | undefined,
  signature: undefined as string | undefined,
  /** v2 phase 6 §5 — stock-in/write-off at/above this value needs a manager PIN. 0/empty = off. */
  inventoryApprovalThreshold: undefined as number | undefined,
  /**
   * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §4): master switches for the three
   * dimensions. All default OFF, so a fresh company looks exactly as it did before this phase until
   * the owner opts in.
   */
  featureBranches: false,
  featureCurrencies: false,
  featureCostCenters: false,
});
const errors = ref<Record<string, string>>({});
const saving = ref(false);
const loading = ref(true);

onMounted(async () => {
  await store.load(true);
  const s = store.settings!;
  Object.assign(form, {
    storeName: s.storeName,
    address: s.address ?? '',
    nationalAddress: s.nationalAddress ?? { country: s.country ?? DEFAULT_COUNTRY, street: s.address || undefined },
    phone: s.phone ?? '',
    commercialRegister: s.commercialRegister ?? '',
    vatNumber: s.vatNumber ?? '',
    currency: s.currency,
    invoiceNumberPrefix: s.invoiceNumberPrefix,
    receiptFooter: s.receiptFooter ?? '',
    defaultTaxId: s.defaultTaxId ?? '',
    pricesIncludeTax: s.pricesIncludeTax !== false,
    logo: s.logo,
    stamp: s.stamp,
    signature: s.signature,
    inventoryApprovalThreshold: s.inventoryApprovalThreshold,
    featureBranches: s.features?.branches ?? false,
    featureCurrencies: s.features?.currencies ?? false,
    featureCostCenters: s.features?.costCenters ?? false,
  });
  currencyLocked.value = await isBaseCurrencyLocked();
  loading.value = false;
});

async function save() {
  errors.value = {};
  if (!form.storeName.trim()) errors.value.storeName = 'اسم المتجر مطلوب';
  if (form.vatNumber && !activeProfile.value.taxId.pattern.test(form.vatNumber)) errors.value.vatNumber = activeProfile.value.taxId.hint;
  if (!form.invoiceNumberPrefix.trim()) errors.value.invoiceNumberPrefix = 'مطلوب';
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    await store.update({
      ...form,
      storeName: form.storeName.trim(),
      address: formatAddress(form.nationalAddress) || form.address.trim() || undefined,
      nationalAddress: form.nationalAddress,
      phone: form.phone.trim() || undefined,
      commercialRegister: form.commercialRegister.trim() || undefined,
      vatNumber: form.vatNumber.trim() || undefined,
      receiptFooter: form.receiptFooter.trim() || undefined,
      defaultTaxId: form.defaultTaxId || undefined,
      pricesIncludeTax: form.pricesIncludeTax,
      stamp: form.stamp,
      signature: form.signature,
      inventoryApprovalThreshold: form.inventoryApprovalThreshold || undefined,
      features: { branches: form.featureBranches, currencies: form.featureCurrencies, costCenters: form.featureCostCenters },
    });
    toast.success('تم حفظ الإعدادات');
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

const outputTaxes = computed(() => store.taxes.filter((t) => t.type === 'OUTPUT').map((t) => ({ value: t.id, label: `${t.name} (${t.rate}%)` })));
</script>

<template>
  <SettingsPage title="الإعدادات" subtitle="بيانات المتجر التي تظهر على الفواتير" wide>
    <template #nav><SettingsTabs /></template>

    <SkeletonBlock v-if="loading" :lines="8" height="h-9" />
    <form v-else class="grid items-start gap-5 xl:grid-cols-[1fr_360px]" novalidate @submit.prevent="save">
      <div class="space-y-5">
        <AppCard title="بيانات المتجر">
          <div class="grid gap-4 sm:grid-cols-2">
            <AppInput v-model="form.storeName" class="sm:col-span-2" label="اسم المتجر" required :disabled="!canWrite" :error="errors.storeName" />
            <AppInput v-model="form.phone" label="الهاتف" ltr :disabled="!canWrite" />
            <AppInput v-model="form.commercialRegister" label="السجل التجاري" ltr :disabled="!canWrite" />
            <AppInput v-model="form.vatNumber" :label="activeProfile.taxId.label" ltr :disabled="!canWrite" :error="errors.vatNumber" :hint="activeProfile.taxId.hint" />
            <AppSelect
              v-model="form.currency"
              label="العملة الأساسية"
              :disabled="!canWrite || currencyLocked"
              :hint="currencyLocked ? 'العملة الأساسية مقفلة بعد بدء الترحيل — لا يمكن تغييرها' : undefined"
              :options="CURRENCY_OPTIONS"
            />
          </div>
        </AppCard>

        <AppCard title="العنوان الوطني">
          <AddressFields v-model="form.nationalAddress" :country="store.settings?.country ?? DEFAULT_COUNTRY" :disabled="!canWrite" />
        </AppCard>

        <AppCard title="الأبعاد (الفروع / العملات / مراكز التكلفة)" subtitle="تبقى واجهات هذه الأبعاد مخفية حتى تُفعّلها — فلا تظهر لأصحاب المتجر الواحد بعملة واحدة">
          <div class="grid gap-3 sm:grid-cols-3">
            <AppSwitch v-model="form.featureBranches" label="تفعيل الفروع" description="مبدّل الفرع في الشريط العلوي، عمود الفرع في المستندات، تقارير الفروع" :disabled="!canWrite" />
            <AppSwitch v-model="form.featureCurrencies" label="تفعيل العملات" description="عملات غير أساسية على العملاء/الموردين/المستندات، فروق العملة" :disabled="!canWrite" />
            <AppSwitch v-model="form.featureCostCenters" label="تفعيل مراكز التكلفة" description="عمود مركز التكلفة في القيود، تقرير الأرباح حسب المركز" :disabled="!canWrite" />
          </div>
        </AppCard>

        <AppCard title="الفواتير">
          <div class="grid gap-4 sm:grid-cols-2">
            <AppInput v-model="form.invoiceNumberPrefix" label="بادئة رقم الفاتورة" ltr :disabled="!canWrite" :error="errors.invoiceNumberPrefix" hint="مثال: INV- تنتج INV-000123" />
            <AppSelect v-model="form.defaultTaxId" label="ضريبة المبيعات الافتراضية" :disabled="!canWrite" :options="outputTaxes" hint="أضف/عدّل الضرائب من تبويب «الضرائب»" />
            <AppInput v-model="form.receiptFooter" class="sm:col-span-2" label="نص أسفل الفاتورة" :disabled="!canWrite" placeholder="مثال: الاستبدال خلال 7 أيام" />
            <AppSwitch
              v-model="form.pricesIncludeTax"
              class="sm:col-span-2"
              label="الأسعار شاملة الضريبة"
              description="الأسعار المُدخلة والخصومات تشمل ضريبة القيمة المضافة (الوضع الافتراضي للبيع بالتجزئة). كل مستند يحفظ نسخته الخاصة من هذا الإعداد."
              :disabled="!canWrite"
            />
          </div>
        </AppCard>

        <AppCard title="المخزون">
          <AppInput
            v-model="form.inventoryApprovalThreshold"
            label="حد اعتماد المدير لحركات المخزون"
            type="number"
            min="0"
            :disabled="!canWrite"
            hint="إدخال مخزون أو إتلاف بقيمة تساوي أو تتجاوز هذا المبلغ يتطلب تأكيد مدير. اتركه فارغاً لتعطيل هذا الشرط."
          />
        </AppCard>
      </div>

      <div class="space-y-5 xl:sticky xl:top-0">
        <AppCard title="الشعار">
          <div class="space-y-2">
            <ImageUploadField v-model="form.logo" label="رفع شعار" change-label="تغيير" alt="شعار المتجر" :disabled="!canWrite" />
            <p class="text-tiny text-text-secondary">PNG أو JPG، حتى 600 كيلوبايت. يُحفظ محلياً.</p>
          </div>
        </AppCard>

        <AppCard title="الختم والتوقيع" subtitle="تُستخدم مستقبلاً على قوالب المستندات المخصصة">
          <div class="space-y-4">
            <ImageUploadField v-model="form.stamp" size="sm" label="رفع ختم" change-label="تغيير الختم" alt="ختم الشركة" :disabled="!canWrite" />
            <ImageUploadField v-model="form.signature" size="sm" label="رفع توقيع" change-label="تغيير التوقيع" alt="توقيع المخول" :disabled="!canWrite" />
            <p class="text-tiny text-text-secondary">PNG بخلفية شفافة يُفضّل، حتى 600 كيلوبايت لكل صورة. تُحفظ محلياً.</p>
          </div>
        </AppCard>

        <AppButton v-if="canWrite" type="submit" variant="primary" block :icon="Save" :loading="saving">حفظ الإعدادات</AppButton>
      </div>
    </form>
  </SettingsPage>
</template>
