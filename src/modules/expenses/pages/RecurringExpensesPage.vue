<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { Pencil, Plus, Repeat, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, formatDate, todayKey } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { deleteRecurringExpense, getExpenseCategories, getRecurringExpenses, saveRecurringExpense } from '../services/expenseService';
import type { ExpenseCategory, RecurringExpense } from '../types';

const toast = useToast();
const confirm = useConfirm();
const settings = useSettingsStore();

const templates = ref<RecurringExpense[]>([]);
const categories = ref<ExpenseCategory[]>([]);
const loading = ref(true);

onMounted(async () => {
  await settings.load();
  [templates.value, categories.value] = await Promise.all([getRecurringExpenses(), getExpenseCategories()]);
  loading.value = false;
});

const categoryOptions = computed(() => categories.value.map((c) => ({ value: c.id, label: c.name })));
const methodOptions = computed(() => settings.paymentMethods.filter((m) => m.active && m.type !== 'credit' && m.type !== 'store_credit').map((m) => ({ value: m.id, label: m.name })));

const formOpen = ref(false);
const editing = ref<RecurringExpense | null>(null);
const saving = ref(false);
const errors = ref<Record<string, string>>({});
const form = reactive({ name: '', categoryId: '', amount: 0 as number | undefined, day: 1, paymentMethodId: '', autoPost: false, active: true, nextDate: todayKey() });

function openCreate() {
  editing.value = null;
  Object.assign(form, { name: '', categoryId: '', amount: undefined, day: 1, paymentMethodId: methodOptions.value[0]?.value ?? '', autoPost: false, active: true, nextDate: todayKey() });
  errors.value = {};
  formOpen.value = true;
}
function openEdit(t: RecurringExpense) {
  editing.value = t;
  Object.assign(form, {
    name: t.name,
    categoryId: t.categoryId,
    amount: t.amount,
    day: t.day,
    paymentMethodId: t.paidFrom.kind === 'method' ? t.paidFrom.paymentMethodId : '',
    autoPost: t.autoPost,
    active: t.active,
    nextDate: t.nextDate,
  });
  errors.value = {};
  formOpen.value = true;
}

async function save() {
  errors.value = {};
  if (!form.name.trim()) errors.value.name = 'الاسم مطلوب';
  if (!form.categoryId) errors.value.categoryId = 'اختر التصنيف';
  if (!(num0(form.amount) > 0)) errors.value.amount = 'أدخل مبلغاً أكبر من صفر';
  if (!form.paymentMethodId) errors.value.paymentMethodId = 'اختر طريقة الدفع';
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    await saveRecurringExpense(
      {
        name: form.name.trim(),
        categoryId: form.categoryId,
        amount: num0(form.amount),
        isTaxInvoice: false,
        paidFrom: { kind: 'method', paymentMethodId: form.paymentMethodId },
        day: form.day,
        nextDate: dateKeyToIso(form.nextDate).slice(0, 10),
        autoPost: form.autoPost,
        active: form.active,
      },
      editing.value?.id,
    );
    templates.value = await getRecurringExpenses();
    toast.success(editing.value ? 'تم التحديث' : 'تمت الإضافة');
    formOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function remove(t: RecurringExpense) {
  const ok = await confirm({ title: `حذف "${t.name}"؟`, danger: true, confirmText: 'حذف' });
  if (!ok) return;
  await deleteRecurringExpense(t.id);
  templates.value = await getRecurringExpenses();
  toast.success('تم الحذف');
}
</script>

<template>
  <div>
    <PageHeader title="المصروفات المتكررة" subtitle="قالب + تاريخ استحقاق تالٍ — يظهر تنبيه عند الاستحقاق في صفحة المصروفات" :back="{ name: 'expenses' }" />
    <SkeletonBlock v-if="loading" :lines="5" height="h-9" />
    <AppCard v-else padding="none">
      <template #actions>
        <AppButton size="sm" :icon="Plus" @click="openCreate">قالب جديد</AppButton>
      </template>
      <EmptyState v-if="!templates.length" :icon="Repeat" title="لا توجد قوالب متكررة" />
      <table v-else class="w-full text-body">
        <thead class="text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="px-4 py-2 text-start font-medium">الاسم</th>
            <th class="px-2 py-2 text-start font-medium">المبلغ</th>
            <th class="px-2 py-2 text-start font-medium">الاستحقاق التالي</th>
            <th class="px-2 py-2 text-start font-medium">نشط</th>
            <th class="px-4 py-2" />
          </tr>
        </thead>
        <tbody>
          <tr v-for="t in templates" :key="t.id" class="border-b border-border last:border-0">
            <td class="px-4 py-2">{{ t.name }}</td>
            <td class="px-2 py-2"><MoneyText :value="t.amount" plain /></td>
            <td class="px-2 py-2"><span class="num text-text-secondary">{{ formatDate(t.nextDate) }}</span></td>
            <td class="px-2 py-2"><span :class="t.active ? 'text-success' : 'text-text-secondary'">{{ t.active ? 'نشط' : 'معطل' }}</span></td>
            <td class="px-4 py-2 text-end">
              <div class="flex justify-end gap-1">
                <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" @click="openEdit(t)"><Pencil class="size-4" /></button>
                <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" @click="remove(t)"><Trash class="size-4" /></button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </AppCard>

    <AppModal v-model:open="formOpen" :title="editing ? 'تعديل قالب' : 'قالب جديد'" :persistent="saving">
      <form class="space-y-4" novalidate @submit.prevent="save">
        <AppInput v-model="form.name" label="الاسم" required :error="errors.name" />
        <div class="grid grid-cols-2 gap-3">
          <AppCombobox v-model="form.categoryId" label="التصنيف" :options="categoryOptions" :error="errors.categoryId" />
          <AppInput v-model.number="form.amount" type="number" min="0" step="0.01" label="المبلغ" :error="errors.amount" />
        </div>
        <div class="grid grid-cols-2 gap-3">
          <AppInput v-model.number="form.day" type="number" min="1" max="28" label="يوم الاستحقاق (من الشهر)" />
          <AppDatePicker v-model="form.nextDate" label="الاستحقاق التالي" />
        </div>
        <AppSelect v-model="form.paymentMethodId" label="طريقة الدفع" :options="methodOptions" :error="errors.paymentMethodId" />
        <div class="grid grid-cols-2 gap-3">
          <AppSwitch v-model="form.autoPost" label="ترحيل تلقائي" description="بدون الحاجة لتأكيد المحاسب" />
          <AppSwitch v-model="form.active" label="نشط" />
        </div>
      </form>
      <template #footer>
        <AppButton :disabled="saving" @click="formOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="saving" @click="save">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
