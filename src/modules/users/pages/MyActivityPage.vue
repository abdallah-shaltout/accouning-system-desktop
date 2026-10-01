<script setup lang="ts">
/**
 * Self-service "نشاطي" — the current user's own slice of the structured audit trail
 * (`db.audit`, read via `getAuditEntries`), reachable by any signed-in user regardless of their
 * `users` area permission (route has no `area` meta). Same data/columns as
 * settings/pages/AuditLogSettingsPage.vue (admin, all users, gated on `users` area) but with the
 * user filter locked to self and no SettingsTabs nav.
 */
import { computed, onMounted, ref, watch } from 'vue';
import { FileClock, ShieldCheck } from '@lucide/vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/modules/core/components/shadcn/dialog';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime } from '@/modules/core/helpers/format';
import { useAuthStore } from '../controllers/useAuthStore';
import AuditDiffView from '@/modules/diagnostics/components/AuditDiffView.vue';
import { getAuditEntities, getAuditEntries } from '@/modules/diagnostics/services/auditService';
import type { AuditAction, AuditEntry } from '@/modules/diagnostics/types';

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

const auth = useAuthStore();
const entity = ref('');
const action = ref<AuditAction | ''>('');
const from = ref('');
const to = ref('');
const search = ref('');
const entities = ref<string[]>([]);
const selected = ref<AuditEntry | null>(null);

const { data, loading, error, reload } = useAsync(() =>
  getAuditEntries({
    userId: auth.user!.id,
    entity: entity.value || undefined,
    action: (action.value || undefined) as AuditAction | undefined,
    from: from.value || undefined,
    to: to.value || undefined,
    search: search.value || undefined,
  }),
);
watch([entity, action, from, to, search], reload);

onMounted(async () => {
  entities.value = await getAuditEntities();
});

const entityOptions = computed(() => [{ value: '', label: 'كل الكيانات' }, ...entities.value.map((e) => ({ value: e, label: e }))]);
const actionOptions = computed(() => [{ value: '', label: 'كل الإجراءات' }, ...Object.entries(ACTION_LABEL).map(([value, label]) => ({ value, label }))]);

const columns: Column<AuditEntry>[] = [
  { key: 'at', label: 'الوقت', sortable: true },
  { key: 'entity', label: 'الكيان', sortable: true },
  { key: 'action', label: 'الإجراء', sortable: true },
  { key: 'message', label: 'التفاصيل' },
];
</script>

<template>
  <div>
    <PageHeader title="نشاطي" subtitle="كل عملية إنشاء أو تعديل أو ترحيل قمت بها" :back="{ name: 'home' }" />

    <div class="mb-3 flex flex-wrap items-center gap-2">
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
      empty-title="لا يوجد نشاط مطابق"
      export-file-name="نشاطي"
      :export-columns="[
        { key: 'at', label: 'الوقت', value: (r: AuditEntry) => formatDateTime(r.at) },
        { key: 'entity', label: 'الكيان' },
        { key: 'entityId', label: 'رقم الكيان' },
        { key: 'action', label: 'الإجراء', value: (r: AuditEntry) => ACTION_LABEL[r.action] },
        { key: 'message', label: 'التفاصيل' },
      ]"
      @retry="reload"
      @row-click="(r) => (selected = r)"
    >
      <template #cell-at="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.at) }}</span></template>
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
  </div>
</template>
