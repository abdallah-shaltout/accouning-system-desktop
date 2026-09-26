<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { AlertTriangle } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import { formatDateTime } from '@/modules/core/helpers/format';
import { groupByFingerprint, loadChannel } from '../../services/diagnosticsReadService';
import type { FingerprintGroup } from '../../types';

/** Errors grouped by fingerprint (18.B5) — same bug counted once instead of flooding the list. */
const groups = ref<FingerprintGroup[]>([]);
const loading = ref(true);

onMounted(async () => {
  const entries = await loadChannel('error', 60);
  groups.value = groupByFingerprint(entries);
  loading.value = false;
});

const columns: Column<FingerprintGroup>[] = [
  { key: 'fingerprint', label: 'الرمز', sortable: true },
  { key: 'name', label: 'النوع', sortable: true },
  { key: 'message', label: 'الرسالة' },
  { key: 'source', label: 'المصدر', sortable: true },
  { key: 'count', label: 'العدد', numeric: true, sortable: true },
  { key: 'lastSeen', label: 'آخر ظهور', sortable: true },
];
</script>

<template>
  <DataTable
    :columns="columns"
    :rows="groups"
    :loading="loading"
    :empty-icon="AlertTriangle"
    empty-title="لا توجد أخطاء مسجّلة"
  >
    <template #cell-fingerprint="{ row }"><span class="num text-xs text-text-secondary">E-{{ row.fingerprint.slice(0, 4).toUpperCase() }}</span></template>
    <template #cell-lastSeen="{ row }"><span class="num text-xs">{{ formatDateTime(row.lastSeen) }}</span></template>
  </DataTable>
</template>
