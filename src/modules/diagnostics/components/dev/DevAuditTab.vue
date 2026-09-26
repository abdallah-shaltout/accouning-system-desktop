<script setup lang="ts">
/** Read-only view of the business audit trail (18.B5) inside `/dev/diagnostics` — the same data
 * as Settings → سجل التدقيق, without the filters, for a quick developer glance while testing. */
import { onMounted, ref } from 'vue';
import { ShieldCheck } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import { formatDateTime } from '@/modules/core/helpers/format';
import { getAuditEntries } from '../../services/auditService';
import type { AuditEntry } from '../../types';

const entries = ref<AuditEntry[]>([]);
const loading = ref(true);

onMounted(async () => {
  entries.value = await getAuditEntries();
  loading.value = false;
});

const columns: Column<AuditEntry>[] = [
  { key: 'at', label: 'الوقت', sortable: true },
  { key: 'entity', label: 'الكيان', sortable: true },
  { key: 'action', label: 'الإجراء', sortable: true },
  { key: 'message', label: 'التفاصيل' },
  { key: 'userId', label: 'المستخدم', sortable: true },
];
</script>

<template>
  <DataTable :columns="columns" :rows="entries" :loading="loading" :empty-icon="ShieldCheck" empty-title="لا توجد قيود تدقيق">
    <template #cell-at="{ row }"><span class="num text-xs text-text-secondary">{{ formatDateTime(row.at) }}</span></template>
  </DataTable>
</template>
