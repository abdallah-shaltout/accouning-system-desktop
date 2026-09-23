<script setup lang="ts">
// TODO(phase 13a, backup track): wire <AttachmentField> onto company logo/stamp/signature uploads
// (docs/v2/14-platform.md §5 — "company settings" branding row).
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
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import { useSettingsStore } from '../controllers/useSettingsStore';

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
  /** v2 (docs/v2/06-sales-and-pos.md §3, README decision 4): default true — Saudi B2C shelf pricing. */
  pricesIncludeTax: true,
  logo: undefined as string | undefined,
  /** v2 phase 6 §5 — stock-in/write-off at/above this value needs a manager PIN. 0/empty = off. */
  inventoryApprovalThreshold: undefined as number | undefined,
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
    pricesIncludeTax: s.pricesIncludeTax !== false,
    logo: s.logo,
    inventoryApprovalThreshold: s.inventoryApprovalThreshold,
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
      pricesIncludeTax: form.pricesIncludeTax,
      inventoryApprovalThreshold: form.inventoryApprovalThreshold || undefined,
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
  <div>
    <PageHeader title="الإعدادات" subtitle="بيانات المتجر التي تظهر على الفواتير" />
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
            <AppSelect v-model="form.defaultTaxId" label="ضريبة المبيعات الافتراضية" :disabled="!canWrite" :options="outputTaxes" hint="أضف/عدّل الضرائب من تبويب «الضرائب»" />
            <AppInput v-model="form.receiptFooter" class="sm:col-span-2" label="نص أسفل الفاتورة" :disabled="!canWrite" placeholder="مثال: الاستبدال خلال 7 أيام" />
            <AppSwitch
              v-model="form.pricesIncludeTax"
              class="sm:col-span-2"
              label="الأسعار شاملة الضريبة"
              description="الأسعار المُدخلة والخصومات تشمل ضريبة القيمة المضافة (الافتراضي للبيع بالتجزئة في السعودية). كل مستند يحفظ نسخته الخاصة من هذا الإعداد."
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
