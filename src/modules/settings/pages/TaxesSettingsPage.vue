<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { Pencil, Plus, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { toNum } from '@/modules/core/helpers/numbers';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import { useSettingsStore } from '../controllers/useSettingsStore';
import { deleteTax, saveTax } from '../services/settingsService';
import type { Tax, TaxCategory } from '../types';

const store = useSettingsStore();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const canWrite = computed(() => auth.can('settings', 'write'));

const CATEGORY_OPTIONS: { value: TaxCategory; label: string }[] = [
  { value: 'S', label: 'قياسية (S) — خاضعة للنسبة الأساسية' },
  { value: 'Z', label: 'صفرية (Z) — نسبة صفر، مثل الأدوية والصادرات' },
  { value: 'E', label: 'معفاة (E) — تتطلب سبب إعفاء' },
  { value: 'O', label: 'خارج النطاق (O) — خارج نظام الضريبة' },
];
const CATEGORY_LABEL: Record<TaxCategory, string> = { S: 'قياسية', Z: 'صفرية', E: 'معفاة', O: 'خارج النطاق' };

const loading = ref(true);
onMounted(async () => {
  await store.load(true);
  loading.value = false;
});

const salesTaxes = computed(() => store.taxes.filter((t) => t.type === 'OUTPUT'));
const purchaseTaxes = computed(() => store.taxes.filter((t) => t.type === 'INPUT'));

// --- form modal ---
const formOpen = ref(false);
const editing = ref<Tax | null>(null);
const saving = ref(false);
const errors = ref<Record<string, string>>({});
const form = reactive({
  name: '',
  rate: 15 as number | undefined,
  type: 'OUTPUT' as 'OUTPUT' | 'INPUT',
  category: 'S' as TaxCategory,
  isDefault: false,
  active: true,
  exemptionReason: '',
});

function openCreate(type: 'OUTPUT' | 'INPUT') {
  editing.value = null;
  Object.assign(form, { name: '', rate: type === 'OUTPUT' ? 15 : 15, type, category: 'S', isDefault: false, active: true, exemptionReason: '' });
  errors.value = {};
  formOpen.value = true;
}

function openEdit(tax: Tax) {
  editing.value = tax;
  Object.assign(form, {
    name: tax.name,
    rate: tax.rate,
    type: tax.type,
    category: tax.category,
    isDefault: tax.isDefault,
    active: tax.active,
    exemptionReason: tax.exemptionReason ?? '',
  });
  errors.value = {};
  formOpen.value = true;
}

async function save() {
  errors.value = {};
  if (!form.name.trim()) errors.value.name = 'اسم الضريبة مطلوب';
  const rate = toNum(form.rate);
  if (rate === undefined || rate < 0 || rate > 100) errors.value.rate = 'نسبة بين 0 و 100';
  if (form.category === 'E' && !form.exemptionReason.trim()) errors.value.exemptionReason = 'سبب الإعفاء مطلوب';
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    await saveTax(
      {
        name: form.name.trim(),
        rate: rate!,
        type: form.type,
        category: form.category,
        isDefault: form.isDefault,
        active: form.active,
        direction: form.type === 'OUTPUT' ? 'sales' : 'purchase',
        exemptionReason: form.category === 'E' ? form.exemptionReason.trim() : undefined,
      },
      editing.value?.id,
    );
    await store.reloadTaxes();
    toast.success(editing.value ? 'تم تحديث الضريبة' : 'تمت إضافة الضريبة');
    formOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function toggleActive(tax: Tax) {
  try {
    await saveTax({ name: tax.name, rate: tax.rate, type: tax.type, category: tax.category, isDefault: tax.isDefault, active: !tax.active, direction: tax.direction, exemptionReason: tax.exemptionReason }, tax.id);
    await store.reloadTaxes();
  } catch (err) {
    toast.error(err);
  }
}

async function remove(tax: Tax) {
  const ok = await confirm({ title: `حذف "${tax.name}"؟`, message: 'لا يمكن التراجع عن هذا الإجراء.', danger: true, confirmText: 'حذف' });
  if (!ok) return;
  try {
    await deleteTax(tax.id);
    await store.reloadTaxes();
    toast.success('تم حذف الضريبة');
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="الضرائب" subtitle="فئات ضريبة القيمة المضافة (قياسية / صفرية / معفاة / خارج النطاق) المستخدمة في المبيعات والمشتريات" />
    <SettingsTabs />

    <SkeletonBlock v-if="loading" :lines="6" height="h-9" />
    <div v-else class="space-y-5">
      <AppCard title="ضرائب المبيعات" padding="none">
        <template #actions>
          <AppButton v-if="canWrite" size="sm" :icon="Plus" @click="openCreate('OUTPUT')">إضافة</AppButton>
        </template>
        <EmptyState v-if="!salesTaxes.length" title="لا توجد ضرائب مبيعات" />
        <table v-else class="w-full text-body">
          <thead class="text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2 text-start font-medium">الضريبة</th>
              <th class="px-2 py-2 text-start font-medium">الفئة</th>
              <th class="px-2 py-2 text-start font-medium">النسبة</th>
              <th class="px-2 py-2 text-start font-medium">نشطة</th>
              <th class="px-4 py-2"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="t in salesTaxes" :key="t.id" class="border-b border-border last:border-0">
              <td class="px-4 py-2">
                {{ t.name }}
                <span v-if="t.isDefault" class="ms-1.5 rounded-full bg-primary/10 px-1.5 text-tiny text-primary">افتراضية</span>
                <span v-if="t.category === 'E' && t.exemptionReason" class="block text-tiny text-text-secondary">{{ t.exemptionReason }}</span>
              </td>
              <td class="px-2 py-2 text-text-secondary">{{ CATEGORY_LABEL[t.category] }}</td>
              <td class="px-2 py-2"><span class="num">{{ t.rate }}%</span></td>
              <td class="px-2 py-2"><AppSwitch :model-value="t.active" :disabled="!canWrite || t.isDefault" @update:model-value="() => toggleActive(t)" /></td>
              <td class="px-4 py-2 text-end">
                <div class="flex justify-end gap-1">
                  <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" :disabled="!canWrite" @click="openEdit(t)">
                    <Pencil class="size-4" />
                  </button>
                  <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" :disabled="!canWrite || t.isDefault" @click="remove(t)">
                    <Trash class="size-4" />
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </AppCard>

      <AppCard title="ضرائب المشتريات" padding="none">
        <template #actions>
          <AppButton v-if="canWrite" size="sm" :icon="Plus" @click="openCreate('INPUT')">إضافة</AppButton>
        </template>
        <EmptyState v-if="!purchaseTaxes.length" title="لا توجد ضرائب مشتريات" />
        <table v-else class="w-full text-body">
          <thead class="text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2 text-start font-medium">الضريبة</th>
              <th class="px-2 py-2 text-start font-medium">الفئة</th>
              <th class="px-2 py-2 text-start font-medium">النسبة</th>
              <th class="px-2 py-2 text-start font-medium">نشطة</th>
              <th class="px-4 py-2"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="t in purchaseTaxes" :key="t.id" class="border-b border-border last:border-0">
              <td class="px-4 py-2">
                {{ t.name }}
                <span v-if="t.isDefault" class="ms-1.5 rounded-full bg-primary/10 px-1.5 text-tiny text-primary">افتراضية</span>
              </td>
              <td class="px-2 py-2 text-text-secondary">{{ CATEGORY_LABEL[t.category] }}</td>
              <td class="px-2 py-2"><span class="num">{{ t.rate }}%</span></td>
              <td class="px-2 py-2"><AppSwitch :model-value="t.active" :disabled="!canWrite || t.isDefault" @update:model-value="() => toggleActive(t)" /></td>
              <td class="px-4 py-2 text-end">
                <div class="flex justify-end gap-1">
                  <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" :disabled="!canWrite" @click="openEdit(t)">
                    <Pencil class="size-4" />
                  </button>
                  <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" :disabled="!canWrite || t.isDefault" @click="remove(t)">
                    <Trash class="size-4" />
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </AppCard>
    </div>

    <AppModal v-model:open="formOpen" :title="editing ? 'تعديل ضريبة' : 'ضريبة جديدة'" :persistent="saving">
      <form class="space-y-4" novalidate @submit.prevent="save">
        <AppInput v-model="form.name" label="الاسم" required :error="errors.name" />
        <div class="grid grid-cols-2 gap-3">
          <AppSelect v-model="form.type" label="النوع" :options="[{ value: 'OUTPUT', label: 'مبيعات (مخرجات)' }, { value: 'INPUT', label: 'مشتريات (مدخلات)' }]" />
          <AppInput v-model="form.rate" type="number" min="0" max="100" label="النسبة %" required :error="errors.rate" />
        </div>
        <AppSelect v-model="form.category" label="الفئة الضريبية (ZATCA)" :options="CATEGORY_OPTIONS" hint="الصفرية والمعفاة كلاهما 0% لكنهما مربعان مختلفان في إقرار الضريبة" />
        <AppTextarea v-if="form.category === 'E'" v-model="form.exemptionReason" label="سبب الإعفاء *" :error="errors.exemptionReason" :rows="2" />
        <AppSwitch v-model="form.isDefault" label="الضريبة الافتراضية لهذا النوع" description="تُستخدم تلقائياً عند عدم تحديد ضريبة للسطر" />
        <AppSwitch v-model="form.active" label="نشطة" />
      </form>
      <template #footer>
        <AppButton :disabled="saving" @click="formOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="saving" @click="save">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
