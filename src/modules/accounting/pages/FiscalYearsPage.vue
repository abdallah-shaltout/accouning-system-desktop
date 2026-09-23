<script setup lang="ts">
import { computed, reactive, ref } from 'vue';
import { CalendarRange, Lock, Pencil, Plus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { formatDate, todayKey } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getFiscalYears, saveFiscalYear } from '../services/accountingService';
import type { FiscalYear } from '../types';

const auth = useAuthStore();
const toast = useToast();
const canWrite = computed(() => auth.can('accounting', 'write'));
const { data, loading, error, reload } = useAsync(getFiscalYears);

const today = todayKey();
const isCurrent = (f: FiscalYear) => f.startDate <= today && f.endDate >= today;

const open = ref(false);
const editing = ref<FiscalYear | null>(null);
const form = reactive({ name: '', startDate: '', endDate: '', isClosed: false });
const formError = ref('');
const saving = ref(false);

function openForm(f?: FiscalYear) {
  editing.value = f ?? null;
  const nextYear = (data.value?.[0] ? Number(data.value[0].name) || new Date().getFullYear() : new Date().getFullYear()) + 1;
  Object.assign(form, f ?? { name: String(nextYear), startDate: `${nextYear}-01-01`, endDate: `${nextYear}-12-31`, isClosed: false });
  formError.value = '';
  open.value = true;
}

async function save() {
  saving.value = true;
  formError.value = '';
  try {
    await saveFiscalYear({ ...form }, editing.value?.id);
    toast.success('تم حفظ السنة المالية', form.name);
    open.value = false;
    reload();
  } catch (err) {
    formError.value = errorMessage(err);
  } finally {
    saving.value = false;
  }
}

const columns: Column<FiscalYear>[] = [
  { key: 'name', label: 'السنة المالية' },
  { key: 'startDate', label: 'من' },
  { key: 'endDate', label: 'إلى' },
  { key: 'isClosed', label: 'الحالة' },
  { key: 'actions', label: '', align: 'end' },
];
</script>

<template>
  <div>
    <PageHeader title="السنة المالية" subtitle="تحدد الفترة الافتراضية لتقارير الحسابات — لا يوجد قفل للفترات في هذه المرحلة">
      <template v-if="canWrite" #actions>
        <AppButton variant="primary" :icon="Plus" @click="openForm()">سنة مالية جديدة</AppButton>
      </template>
    </PageHeader>

    <DataTable :columns="columns" :rows="data" :loading="loading" :error="error" :page-size="0" :empty-icon="CalendarRange" @retry="reload">
      <template #cell-name="{ row }">
        <span class="font-medium">{{ row.name }}</span>
        <StatusBadge v-if="isCurrent(row)" class="ms-2" tone="primary" label="الحالية" />
      </template>
      <template #cell-startDate="{ row }"><span class="num">{{ formatDate(row.startDate) }}</span></template>
      <template #cell-endDate="{ row }"><span class="num">{{ formatDate(row.endDate) }}</span></template>
      <template #cell-isClosed="{ row }">
        <span v-if="row.isClosed" class="inline-flex items-center gap-1 text-xs text-text-secondary"><Lock class="size-3" /> مقفلة</span>
        <StatusBadge v-else tone="success" label="مفتوحة" />
      </template>
      <template #cell-actions="{ row }">
        <AppButton v-if="canWrite" size="sm" variant="ghost" :icon="Pencil" @click="openForm(row)">تعديل</AppButton>
      </template>
    </DataTable>

    <AppModal v-model:open="open" :title="editing ? `تعديل السنة ${editing.name}` : 'سنة مالية جديدة'" size="sm">
      <form id="fy-form" class="space-y-4" @submit.prevent="save">
        <AppInput v-model="form.name" label="الاسم" required />
        <div class="grid grid-cols-2 gap-3">
          <AppInput v-model="form.startDate" type="date" label="تاريخ البداية" required />
          <AppInput v-model="form.endDate" type="date" label="تاريخ النهاية" required />
        </div>
        <AppSwitch v-model="form.isClosed" label="مقفلة" description="علامة للعرض فقط في هذه المرحلة" />
        <p v-if="formError" class="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger">{{ formError }}</p>
      </form>
      <template #footer>
        <AppButton @click="open = false">إلغاء</AppButton>
        <AppButton type="submit" form="fy-form" variant="primary" :loading="saving">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
