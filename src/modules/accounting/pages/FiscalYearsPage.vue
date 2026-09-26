<script setup lang="ts">
/**
 * Fiscal years + closing wizard (docs/v2/02-accounting-review.md B2, 11-journal-dashboard-
 * insights.md A4): pre-checks (no drafts, trial balance balanced, 3900 = 0), the closing entry
 * (revenue+expenses → retained earnings 3250), lock the year, open the next one. Reopen reverses
 * the closing entry — admin-only, same coarse `role === 'admin'` pattern Phase 1 used for the
 * closed-period posting override.
 */
import { computed, onMounted, reactive, ref } from 'vue';
import { CalendarRange, CheckCircle2, CircleX, Lock, LockOpen, Pencil, Plus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { formatDate, todayKey } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import {
  closeYear,
  getCloseYearPreChecks,
  getFiscalYears,
  getLockDate,
  reopenYear,
  saveFiscalYear,
  saveLockDate,
  type CloseYearPreCheck,
} from '../services/accountingService';
import type { FiscalYear } from '../types';

const auth = useAuthStore();
const toast = useToast();
const canWrite = computed(() => auth.can('accounting', 'write'));
const isAdmin = computed(() => auth.role === 'admin');
const { data, loading, error, reload } = useAsync(getFiscalYears);

// Lock date (B2): no posting is allowed on/before this date, regardless of the fiscal year's own
// open/closed flag.
const lockDate = ref('');
const lockSaving = ref(false);
onMounted(async () => (lockDate.value = (await getLockDate()) ?? ''));
async function saveLock() {
  lockSaving.value = true;
  try {
    await saveLockDate(lockDate.value || undefined);
    toast.success('تم حفظ تاريخ القفل');
  } catch (err) {
    toast.error(err);
  } finally {
    lockSaving.value = false;
  }
}

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

// --- Closing wizard --------------------------------------------------------------------------

const wizardOpen = ref(false);
const wizardYear = ref<FiscalYear | null>(null);
const wizardStep = ref<'checks' | 'confirm' | 'done'>('checks');
const preChecks = ref<CloseYearPreCheck[]>([]);
const checksLoading = ref(false);
const closing = ref(false);
const closeResult = ref<{ closingEntryId: string; nextYearName?: string } | null>(null);

async function openWizard(f: FiscalYear) {
  wizardYear.value = f;
  wizardStep.value = 'checks';
  closeResult.value = null;
  wizardOpen.value = true;
  checksLoading.value = true;
  try {
    preChecks.value = await getCloseYearPreChecks(f.id);
  } catch (err) {
    toast.error(err);
  } finally {
    checksLoading.value = false;
  }
}

const allChecksPassed = computed(() => preChecks.value.length > 0 && preChecks.value.every((c) => c.passed));

async function confirmClose() {
  if (!wizardYear.value) return;
  closing.value = true;
  try {
    const result = await closeYear(wizardYear.value.id);
    closeResult.value = { closingEntryId: result.closingEntry.id, nextYearName: result.nextYear?.name };
    wizardStep.value = 'done';
    toast.success(`تم إقفال السنة المالية ${wizardYear.value.name}`);
    reload();
  } catch (err) {
    toast.error(err);
  } finally {
    closing.value = false;
  }
}

const confirm = useConfirm();
async function reopen(f: FiscalYear) {
  const ok = await confirm({
    title: `إعادة فتح السنة المالية ${f.name}؟`,
    message: 'سيتم عكس قيد الإقفال وإلغاء قفل السنة. هذا الإجراء للمدير فقط.',
    confirmText: 'إعادة الفتح',
    danger: true,
  });
  if (!ok) return;
  try {
    await reopenYear(f.id);
    toast.success('تم إعادة فتح السنة المالية');
    reload();
  } catch (err) {
    toast.error(err);
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
    <PageHeader title="السنة المالية" subtitle="تحدد الفترة الافتراضية لتقارير الحسابات — استخدم معالج الإقفال لإقفال سنة بالكامل">
      <template v-if="canWrite" #actions>
        <AppButton variant="primary" :icon="Plus" @click="openForm()">سنة مالية جديدة</AppButton>
      </template>
    </PageHeader>

    <div v-if="canWrite" class="mb-4 flex flex-wrap items-end gap-3 rounded-xl border border-border p-3">
      <AppDatePicker v-model="lockDate" label="تاريخ قفل الترحيل" hint="لا يمكن الترحيل في تاريخ يساويه أو يسبقه إلا بصلاحية المدير" class="w-52" />
      <AppButton :loading="lockSaving" @click="saveLock">حفظ تاريخ القفل</AppButton>
      <AppButton v-if="lockDate" variant="ghost" @click="((lockDate = ''), saveLock())">إزالة القفل</AppButton>
    </div>

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
        <div class="flex justify-end gap-1">
          <AppButton v-if="canWrite && !row.isClosed" size="sm" variant="ghost" :icon="Pencil" @click="openForm(row)">تعديل</AppButton>
          <AppButton v-if="canWrite && !row.isClosed" size="sm" variant="ghost" :icon="Lock" @click="openWizard(row)">إقفال السنة</AppButton>
          <AppButton v-if="isAdmin && row.isClosed" size="sm" variant="ghost" :icon="LockOpen" @click="reopen(row)">إعادة فتح</AppButton>
        </div>
      </template>
    </DataTable>

    <AppModal v-model:open="open" :title="editing ? `تعديل السنة ${editing.name}` : 'سنة مالية جديدة'" size="sm">
      <form id="fy-form" class="space-y-4" @submit.prevent="save">
        <AppInput v-model="form.name" label="الاسم" required />
        <div class="grid grid-cols-2 gap-3">
          <AppDatePicker v-model="form.startDate" label="تاريخ البداية" required />
          <AppDatePicker v-model="form.endDate" label="تاريخ النهاية" required />
        </div>
        <AppSwitch v-model="form.isClosed" label="مقفلة" description="علامة للعرض فقط — استخدم معالج الإقفال للإقفال الفعلي" />
        <p v-if="formError" class="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger">{{ formError }}</p>
      </form>
      <template #footer>
        <AppButton @click="open = false">إلغاء</AppButton>
        <AppButton type="submit" form="fy-form" variant="primary" :loading="saving">حفظ</AppButton>
      </template>
    </AppModal>

    <!-- Closing wizard -->
    <AppModal v-model:open="wizardOpen" :title="`إقفال السنة المالية ${wizardYear?.name ?? ''}`" size="md" persistent>
      <div v-if="wizardStep === 'checks'" class="space-y-4">
        <p class="text-body text-text-secondary">يجب اجتياز الفحوصات التالية قبل إقفال السنة:</p>
        <div v-if="checksLoading" class="space-y-2">
          <div v-for="i in 3" :key="i" class="h-10 animate-shimmer rounded-lg bg-surface-hover" />
        </div>
        <ul v-else class="space-y-2">
          <li v-for="c in preChecks" :key="c.key" class="flex items-start gap-2 rounded-lg border border-border p-3">
            <CheckCircle2 v-if="c.passed" class="mt-0.5 size-4 shrink-0 text-success" />
            <CircleX v-else class="mt-0.5 size-4 shrink-0 text-danger" />
            <div>
              <p class="text-body font-medium">{{ c.label }}</p>
              <p class="text-xs text-text-secondary">{{ c.detail }}</p>
            </div>
          </li>
        </ul>
      </div>

      <div v-else-if="wizardStep === 'confirm'" class="space-y-3">
        <p class="text-body">
          سيتم نشر قيد إقفال (الإيرادات والمصروفات → الأرباح المحتجزة)، ثم قفل السنة المالية
          <strong>{{ wizardYear?.name }}</strong>، وفتح السنة التالية تلقائياً إن لم تكن موجودة.
        </p>
        <p class="text-body text-danger">لا يمكن التراجع عن هذا الإجراء إلا بإعادة الفتح (صلاحية المدير).</p>
      </div>

      <div v-else-if="wizardStep === 'done'" class="space-y-3 text-center">
        <CheckCircle2 class="mx-auto size-10 text-success" />
        <p class="text-body font-medium">تم إقفال السنة المالية بنجاح</p>
        <p v-if="closeResult?.nextYearName" class="text-body text-text-secondary">تم فتح السنة المالية {{ closeResult.nextYearName }}</p>
        <RouterLink v-if="closeResult" :to="`/accounting/journal/${closeResult.closingEntryId}`" class="text-primary hover:underline">عرض قيد الإقفال</RouterLink>
      </div>

      <template #footer>
        <template v-if="wizardStep === 'checks'">
          <AppButton @click="wizardOpen = false">إلغاء</AppButton>
          <AppButton variant="primary" :disabled="!allChecksPassed" @click="wizardStep = 'confirm'">التالي</AppButton>
        </template>
        <template v-else-if="wizardStep === 'confirm'">
          <AppButton @click="wizardStep = 'checks'">رجوع</AppButton>
          <AppButton variant="danger" :loading="closing" @click="confirmClose">تأكيد الإقفال</AppButton>
        </template>
        <template v-else>
          <AppButton variant="primary" @click="wizardOpen = false">إغلاق</AppButton>
        </template>
      </template>
    </AppModal>
  </div>
</template>
