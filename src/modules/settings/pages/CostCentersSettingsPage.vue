<script setup lang="ts">
/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §3): Settings → Accounting → Cost
 * centers. Branch cost centers show read-only (created/deleted only alongside their branch).
 */
import { computed, onMounted, reactive, ref } from 'vue';
import { Pencil, Plus, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import { createCostCenter, deleteCostCenter, getCostCenters, updateCostCenter } from '../services/branchesService';
import type { CostCenter, CostCenterType } from '../types';

const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const canWrite = computed(() => auth.can('settings', 'write'));

const TYPE_LABEL: Record<CostCenterType, string> = { branch: 'فرع', department: 'قسم', project: 'مشروع', other: 'أخرى' };
const TYPE_OPTIONS = Object.entries(TYPE_LABEL).map(([value, label]) => ({ value, label }));

const loading = ref(true);
const centers = ref<CostCenter[]>([]);

async function reload() {
  centers.value = await getCostCenters();
}
onMounted(async () => {
  await reload();
  loading.value = false;
});

const parentOptions = computed(() => [{ value: '', label: 'بدون (رئيسي)' }, ...centers.value.map((c) => ({ value: c.id, label: c.name }))]);

const formOpen = ref(false);
const editing = ref<CostCenter | null>(null);
const saving = ref(false);
const errors = ref<Record<string, string>>({});
const form = reactive({ name: '', code: '', type: 'department' as CostCenterType, parentId: '', active: true });

function openCreate() {
  editing.value = null;
  Object.assign(form, { name: '', code: '', type: 'department', parentId: '', active: true });
  errors.value = {};
  formOpen.value = true;
}

function openEdit(cc: CostCenter) {
  editing.value = cc;
  Object.assign(form, { name: cc.name, code: cc.code, type: cc.type, parentId: cc.parentId ?? '', active: cc.active });
  errors.value = {};
  formOpen.value = true;
}

async function save() {
  errors.value = {};
  if (!form.name.trim()) errors.value.name = 'الاسم مطلوب';
  if (!form.code.trim()) errors.value.code = 'الرمز مطلوب';
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    const payload = { name: form.name.trim(), code: form.code.trim(), type: form.type, parentId: form.parentId || undefined, active: form.active };
    if (editing.value) await updateCostCenter(editing.value.id, payload);
    else await createCostCenter(payload);
    await reload();
    toast.success(editing.value ? 'تم التحديث' : 'تمت الإضافة');
    formOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function remove(cc: CostCenter) {
  const ok = await confirm({ title: `حذف "${cc.name}"؟`, danger: true, confirmText: 'حذف' });
  if (!ok) return;
  try {
    await deleteCostCenter(cc.id);
    await reload();
    toast.success('تم الحذف');
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="مراكز التكلفة" subtitle="تُنشأ مراكز الفروع تلقائياً ولا يمكن حذفها؛ استخدم «توزيع» في القيد اليدوي لتقسيم سطر على عدة مراكز" />
    <SettingsTabs />

    <SkeletonBlock v-if="loading" :lines="4" height="h-12" />
    <AppCard v-else padding="none">
      <template #actions>
        <AppButton v-if="canWrite" size="sm" :icon="Plus" @click="openCreate">إضافة مركز تكلفة</AppButton>
      </template>
      <EmptyState v-if="!centers.length" title="لا توجد مراكز تكلفة" />
      <table v-else class="w-full text-body">
        <thead class="text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="px-4 py-2 text-start font-medium">المركز</th>
            <th class="px-2 py-2 text-start font-medium">الرمز</th>
            <th class="px-2 py-2 text-start font-medium">النوع</th>
            <th class="px-2 py-2 text-start font-medium">نشط</th>
            <th class="px-4 py-2"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="c in centers" :key="c.id" class="border-b border-border last:border-0">
            <td class="px-4 py-2 font-medium">{{ c.name }}</td>
            <td class="px-2 py-2"><span class="num text-text-secondary">{{ c.code }}</span></td>
            <td class="px-2 py-2 text-text-secondary">{{ TYPE_LABEL[c.type] }}</td>
            <td class="px-2 py-2"><AppSwitch :model-value="c.active" disabled /></td>
            <td class="px-4 py-2 text-end">
              <div class="flex justify-end gap-1">
                <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" :disabled="!canWrite" @click="openEdit(c)">
                  <Pencil class="size-4" />
                </button>
                <button
                  v-if="c.canDelete"
                  type="button"
                  class="rounded-md p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger"
                  :disabled="!canWrite"
                  @click="remove(c)"
                >
                  <Trash class="size-4" />
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </AppCard>

    <AppModal v-model:open="formOpen" :title="editing ? 'تعديل مركز تكلفة' : 'مركز تكلفة جديد'" :persistent="saving">
      <form class="space-y-4" novalidate @submit.prevent="save">
        <AppInput v-model="form.name" label="الاسم" required :error="errors.name" />
        <AppInput v-model="form.code" label="الرمز" required ltr :error="errors.code" />
        <AppSelect v-model="form.type" label="النوع" :options="TYPE_OPTIONS" />
        <AppSelect v-model="form.parentId" label="المركز الأب" :options="parentOptions" />
        <AppSwitch v-model="form.active" label="نشط" />
      </form>
      <template #footer>
        <AppButton :disabled="saving" @click="formOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="saving" @click="save">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
