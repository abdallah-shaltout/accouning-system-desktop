<script setup lang="ts">
/**
 * Templates + recurring entries (docs/v2/11-journal-dashboard-insights.md A2/A3). No scheduler
 * here — "due" recurring templates (nextDate <= today) get a badge and a manual "ترحيل الآن" post
 * action. TODO(phase 10): surface these as a dashboard/insight-engine card instead of requiring a
 * visit to this page (see D2 "Recurring due" in the same doc).
 */
import { computed, ref } from 'vue';
import { Plus, Repeat, Send, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDate, todayKey } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getJournalTemplates, postRecurringTemplate, removeJournalTemplate } from '../services/accountingService';
import type { JournalTemplate } from '../types';

const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const { data, loading, error, reload } = useAsync(getJournalTemplates);

const RECURRENCE_LABEL = { month: 'شهرياً', quarter: 'ربع سنوي', year: 'سنوياً' } as const;

function isDue(t: JournalTemplate): boolean {
  return !!t.recurrence && t.recurrence.nextDate <= todayKey();
}

const posting = ref<string | null>(null);
async function postNow(t: JournalTemplate) {
  posting.value = t.id;
  try {
    const entry = await postRecurringTemplate(t.id);
    toast.success('تم ترحيل القيد المتكرر', entry.number);
    reload();
  } catch (err) {
    toast.error(err);
  } finally {
    posting.value = null;
  }
}

async function remove(t: JournalTemplate) {
  const ok = await confirm({ title: `حذف القالب "${t.name}"؟`, confirmText: 'حذف', danger: true });
  if (!ok) return;
  try {
    await removeJournalTemplate(t.id);
    toast.success('تم حذف القالب');
    reload();
  } catch (err) {
    toast.error(err);
  }
}

const columns: Column<JournalTemplate>[] = [
  { key: 'name', label: 'القالب' },
  { key: 'lines', label: 'الأسطر', numeric: true },
  { key: 'recurrence', label: 'التكرار' },
  { key: 'actions', label: '', align: 'end' },
];

const dueCount = computed(() => (data.value ?? []).filter(isDue).length);
</script>

<template>
  <div>
    <PageHeader title="قوالب القيود والقيود المتكررة" subtitle="قوالب جاهزة (إيجار، رواتب، إهلاك…) — احفظ قالباً من نموذج قيد يومية جديد">
      <template v-if="auth.can('accounting', 'write')" #actions>
        <AppButton variant="primary" :icon="Plus" to="/accounting/journal/new">قيد جديد لحفظه كقالب</AppButton>
      </template>
    </PageHeader>

    <p v-if="dueCount" class="mb-3 rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-body text-warning">
      {{ dueCount }} قيد متكرر بحاجة للترحيل اليوم
    </p>

    <DataTable :columns="columns" :rows="data" :loading="loading" :error="error" :page-size="0" :empty-icon="Repeat" @retry="reload">
      <template #cell-name="{ row }">
        <span class="font-medium">{{ row.name }}</span>
        <p v-if="row.description" class="text-xs text-text-secondary">{{ row.description }}</p>
      </template>
      <template #cell-lines="{ row }">{{ row.lines.length }}</template>
      <template #cell-recurrence="{ row }">
        <span v-if="!row.recurrence" class="text-xs text-text-secondary">—</span>
        <div v-else class="flex items-center gap-1.5">
          <StatusBadge tone="neutral" :label="RECURRENCE_LABEL[row.recurrence.every]" />
          <span class="num text-xs text-text-secondary">التالي: {{ formatDate(row.recurrence.nextDate) }}</span>
          <StatusBadge v-if="isDue(row)" tone="warning" label="مستحق" />
        </div>
      </template>
      <template #cell-actions="{ row }">
        <div class="flex justify-end gap-1">
          <AppButton v-if="row.recurrence" size="sm" variant="ghost" :icon="Send" :loading="posting === row.id" @click="postNow(row)">ترحيل الآن</AppButton>
          <AppButton v-if="auth.can('accounting', 'write')" size="sm" variant="ghost" :icon="Trash" @click="remove(row)">حذف</AppButton>
        </div>
      </template>
    </DataTable>
  </div>
</template>
