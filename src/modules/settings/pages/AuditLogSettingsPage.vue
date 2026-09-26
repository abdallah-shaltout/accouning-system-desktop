<script setup lang="ts">
/**
 * Settings → سجل التدقيق (18.B4): read-only view of the structured business audit trail
 * (`db.audit`, written by `logActivity`/`logAudit` in `mocks/backend/core.ts`). Admin only — gated
 * by the route's `area: 'users'` meta, same gate as "المستخدمون والأدوار".
 */
import { computed, onMounted, ref, watch } from 'vue';
import { FileClock, ShieldCheck } from '@lucide/vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import SettingsPage from '@/modules/core/components/layouts/SettingsPage.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/modules/core/components/shadcn/dialog';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime } from '@/modules/core/helpers/format';
import { getUsers } from '@/modules/users/services/userService';
import type { User } from '@/modules/users/types';
import AuditDiffView from '@/modules/diagnostics/components/AuditDiffView.vue';
import { getAuditEntities, getAuditEntries } from '@/modules/diagnostics/services/auditService';
import type { AuditAction, AuditEntry } from '@/modules/diagnostics/types';
import SettingsTabs from '../components/SettingsTabs.vue';

const ACTION_LABEL: Record<AuditAction, string> = {
  create: 'إنشاء',
  update: 'تعديل',
  post: 'ترحيل',
  void: 'إلغاء',
  reverse: 'عكس',
  delete: 'حذف',
  login: 'دخول',
  settings: 'إعدادات',
};

const userId = ref('');
const entity = ref('');
const action = ref<AuditAction | ''>('');
const from = ref('');
const to = ref('');
const search = ref('');
const users = ref<User[]>([]);
const entities = ref<string[]>([]);
const selected = ref<AuditEntry | null>(null);

const { data, loading, error, reload } = useAsync(() =>
  getAuditEntries({
    userId: userId.value || undefined,
    entity: entity.value || undefined,
    action: (action.value || undefined) as AuditAction | undefined,
    from: from.value || undefined,
    to: to.value || undefined,
    search: search.value || undefined,
  }),
);
watch([userId, entity, action, from, to, search], reload);

onMounted(async () => {
  users.value = await getUsers();
  entities.value = await getAuditEntities();
});

const userOptions = computed(() => [{ value: '', label: 'كل المستخدمين' }, ...users.value.map((u) => ({ value: u.id, label: u.name }))]);
const entityOptions = computed(() => [{ value: '', label: 'كل الكيانات' }, ...entities.value.map((e) => ({ value: e, label: e }))]);
const actionOptions = computed(() => [{ value: '', label: 'كل الإجراءات' }, ...Object.entries(ACTION_LABEL).map(([value, label]) => ({ value, label }))]);

function userName(id: string): string {
  return users.value.find((u) => u.id === id)?.name ?? id;
}

const columns: Column<AuditEntry>[] = [
  { key: 'at', label: 'الوقت', sortable: true },
  { key: 'userId', label: 'المستخدم', sortable: true },
  { key: 'entity', label: 'الكيان', sortable: true },
  { key: 'action', label: 'الإجراء', sortable: true },
  { key: 'message', label: 'التفاصيل' },
];
</script>

<template>
  <SettingsPage title="سجل التدقيق" subtitle="سجل تدقيق كامل لكل عملية إنشاء أو تعديل أو ترحيل على بيانات المنشأة" wide>
    <template #nav><SettingsTabs /></template>

    <div class="mb-3 flex flex-wrap items-center gap-2">
      <AppSelect v-model="userId" :options="userOptions" />
      <AppSelect v-model="entity" :options="entityOptions" />
      <AppSelect v-model="action" :options="actionOptions" />
      <DateRangeFilter v-model:from="from" v-model:to="to" />
      <SearchInput v-model="search" placeholder="البحث في التفاصيل أو رقم المستند" class="ms-auto" />
    </div>

    <DataTable
      :columns="columns"
      :rows="data ?? []"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="ShieldCheck"
      empty-title="لا توجد قيود تدقيق مطابقة"
      export-file-name="سجل التدقيق"
      :export-columns="[
        { key: 'at', label: 'الوقت', value: (r: AuditEntry) => formatDateTime(r.at) },
        { key: 'userId', label: 'المستخدم', value: (r: AuditEntry) => userName(r.userId) },
        { key: 'entity', label: 'الكيان' },
        { key: 'entityId', label: 'رقم الكيان' },
        { key: 'action', label: 'الإجراء', value: (r: AuditEntry) => ACTION_LABEL[r.action] },
        { key: 'message', label: 'التفاصيل' },
      ]"
      @retry="reload"
      @row-click="(r) => (selected = r)"
    >
      <template #cell-at="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.at) }}</span></template>
      <template #cell-userId="{ row }">{{ userName(row.userId) }}</template>
      <template #cell-action="{ row }">{{ ACTION_LABEL[row.action] }}</template>
      <template #cell-message="{ row }"><span class="truncate">{{ row.message }}</span></template>
    </DataTable>

    <Dialog :open="!!selected" @update:open="(o) => !o && (selected = null)">
      <DialogContent>
        <DialogHeader>
          <DialogTitle class="flex items-center gap-2"><FileClock class="size-4" /> تفاصيل قيد التدقيق</DialogTitle>
        </DialogHeader>
        <AuditDiffView v-if="selected" :entry="selected" />
      </DialogContent>
    </Dialog>
  </SettingsPage>
</template>
