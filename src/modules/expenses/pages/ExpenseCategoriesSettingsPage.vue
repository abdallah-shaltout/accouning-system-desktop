<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { Pencil, Plus, Receipt, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { accountPath, getAccounts } from '@/modules/accounting/services/accountingService';
import type { AccountWithBalance } from '@/modules/accounting/services/accountingService';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { deleteExpenseCategory, getExpenseCategories, saveExpenseCategory } from '../services/expenseService';
import type { ExpenseCategory } from '../types';

const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const canWrite = computed(() => auth.can('settings', 'write'));

const categories = ref<ExpenseCategory[]>([]);
const accounts = ref<AccountWithBalance[]>([]);
const loading = ref(true);

onMounted(async () => {
  [categories.value, accounts.value] = await Promise.all([getExpenseCategories(), getAccounts()]);
  loading.value = false;
});

const accountOptions = computed(() =>
  accounts.value.filter((a) => a.active && !a.isGroup && a.allowManual).map((a) => ({ value: a.id, label: `${a.code} — ${a.name}`, sublabel: accountPath(a, accounts.value) })),
);

const formOpen = ref(false);
const editing = ref<ExpenseCategory | null>(null);
const saving = ref(false);
const errors = ref<Record<string, string>>({});
const form = reactive({ name: '', accountId: '', active: true });

function openCreate() {
  editing.value = null;
  Object.assign(form, { name: '', accountId: '', active: true });
  errors.value = {};
  formOpen.value = true;
}
function openEdit(c: ExpenseCategory) {
  editing.value = c;
  Object.assign(form, { name: c.name, accountId: c.accountId, active: c.active });
  errors.value = {};
  formOpen.value = true;
}

async function save() {
  errors.value = {};
  if (!form.name.trim()) errors.value.name = 'الاسم مطلوب';
  if (!form.accountId) errors.value.accountId = 'اختر الحساب';
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    await saveExpenseCategory({ name: form.name.trim(), accountId: form.accountId, active: form.active }, editing.value?.id);
    categories.value = await getExpenseCategories();
    toast.success(editing.value ? 'تم تحديث التصنيف' : 'تمت إضافة التصنيف');
    formOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function remove(c: ExpenseCategory) {
  const ok = await confirm({ title: `حذف "${c.name}"؟`, danger: true, confirmText: 'حذف' });
  if (!ok) return;
  try {
    await deleteExpenseCategory(c.id);
    categories.value = await getExpenseCategories();
    toast.success('تم الحذف');
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="تصنيفات المصروفات" subtitle="Settings → Expenses — الاسم والحساب المحاسبي وضريبة افتراضية اختيارية" />
    <SkeletonBlock v-if="loading" :lines="6" height="h-9" />
    <AppCard v-else padding="none">
      <template #actions>
        <AppButton v-if="canWrite" size="sm" :icon="Plus" @click="openCreate">إضافة تصنيف</AppButton>
      </template>
      <EmptyState v-if="!categories.length" :icon="Receipt" title="لا توجد تصنيفات مصروفات" />
      <table v-else class="w-full text-body">
        <thead class="text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="px-4 py-2 text-start font-medium">الاسم</th>
            <th class="px-2 py-2 text-start font-medium">الحساب</th>
            <th class="px-2 py-2 text-start font-medium">نشط</th>
            <th class="px-4 py-2" />
          </tr>
        </thead>
        <tbody>
          <tr v-for="c in categories" :key="c.id" class="border-b border-border last:border-0">
            <td class="px-4 py-2">{{ c.name }}</td>
            <td class="px-2 py-2 text-text-secondary">{{ accounts.find((a) => a.id === c.accountId)?.name ?? '—' }}</td>
            <td class="px-2 py-2"><span :class="c.active ? 'text-success' : 'text-text-secondary'">{{ c.active ? 'نشط' : 'معطل' }}</span></td>
            <td class="px-4 py-2 text-end">
              <div class="flex justify-end gap-1">
                <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" :disabled="!canWrite" @click="openEdit(c)">
                  <Pencil class="size-4" />
                </button>
                <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" :disabled="!canWrite || !c.canDelete" @click="remove(c)">
                  <Trash class="size-4" />
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </AppCard>

    <AppModal v-model:open="formOpen" :title="editing ? 'تعديل تصنيف' : 'تصنيف جديد'" :persistent="saving">
      <form class="space-y-4" novalidate @submit.prevent="save">
        <AppInput v-model="form.name" label="الاسم" required :error="errors.name" />
        <AppCombobox v-model="form.accountId" label="الحساب المحاسبي" required :options="accountOptions" :error="errors.accountId" />
        <AppSwitch v-model="form.active" label="نشط" />
      </form>
      <template #footer>
        <AppButton :disabled="saving" @click="formOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="saving" @click="save">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
