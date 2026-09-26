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
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import SettingsPage from '@/modules/core/components/layouts/SettingsPage.vue';
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

const columns: Column<CostCenter>[] = [
  { key: 'name', label: 'المركز' },
  { key: 'code', label: 'الرمز' },
  { key: 'type', label: 'النوع' },
  { key: 'active', label: 'نشط' },
  { key: 'actions', label: '', type: 'actions' },
];

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
  <SettingsPage title="مراكز التكلفة" subtitle="تُنشأ مراكز الفروع تلقائياً ولا يمكن حذفها؛ استخدم «توزيع» في القيد اليدوي لتقسيم سطر على عدة مراكز">
    <template #nav><SettingsTabs /></template>

    <SkeletonBlock v-if="loading" :lines="4" height="h-12" />
    <AppCard v-else padding="none">
      <template #actions>
        <AppButton v-if="canWrite" size="sm" :icon="Plus" @click="openCreate">إضافة مركز تكلفة</AppButton>
      </template>
      <DataTable :columns="columns" :rows="centers" :page-size="0" empty-title="لا توجد مراكز تكلفة">
        <template #cell-name="{ row }"><span class="font-medium">{{ row.name }}</span></template>
        <template #cell-code="{ row }"><span class="num text-text-secondary">{{ row.code }}</span></template>
        <template #cell-type="{ row }"><span class="text-text-secondary">{{ TYPE_LABEL[row.type] }}</span></template>
        <template #cell-active="{ row }"><AppSwitch :model-value="row.active" disabled /></template>
        <template #cell-actions="{ row }">
          <div class="flex justify-end gap-1">
            <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" :disabled="!canWrite" @click="openEdit(row)">
              <Pencil class="size-4" />
            </button>
            <button
              v-if="row.canDelete"
              type="button"
              class="rounded-md p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger"
              :disabled="!canWrite"
              @click="remove(row)"
            >
              <Trash class="size-4" />
            </button>
          </div>
        </template>
      </DataTable>
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
  </SettingsPage>
</template>
