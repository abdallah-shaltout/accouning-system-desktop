<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { Plus, Wallet } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { daysAgoKey, formatDateTime, formatNumber, todayKey } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getJournalEntries, type JournalRow } from '../services/accountingService';

const router = useRouter();
const auth = useAuthStore();

const type = ref<'all' | 'SYSTEM' | 'MANUAL'>('all');
const search = ref('');
const from = ref(daysAgoKey(29));
const to = ref(todayKey());

const { data, loading, error, reload } = useAsync(() =>
  getJournalEntries({ from: from.value || undefined, to: to.value || undefined }),
);
watch([from, to], reload);

const rows = computed(() => {
  const q = search.value.trim().toLowerCase();
  return (data.value ?? []).filter(
    (e) =>
      (type.value === 'all' || e.type === type.value) &&
      (!q || `${e.number} ${e.description} ${e.sourceRef?.number ?? ''}`.toLowerCase().includes(q)),
  );
});

const typeOptions = computed(() => [
  { value: 'all' as const, label: 'الكل', count: data.value?.length },
  { value: 'SYSTEM' as const, label: 'آلية', count: data.value?.filter((e) => e.type === 'SYSTEM').length },
  { value: 'MANUAL' as const, label: 'يدوية', count: data.value?.filter((e) => e.type === 'MANUAL').length },
]);

const columns: Column<JournalRow>[] = [
  { key: 'number', label: 'رقم القيد', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'description', label: 'البيان' },
  { key: 'type', label: 'النوع' },
  { key: 'lines', label: 'الأسطر', numeric: true },
  { key: 'totalDebit', label: 'المبلغ', numeric: true, sortable: true },
];
</script>

<template>
  <div>
    <PageHeader title="القيود اليومية" subtitle="القيود الآلية الناتجة عن المبيعات والمشتريات والسندات والمخزون، والقيود اليدوية">
      <template v-if="auth.can('accounting', 'write')" #actions>
        <AppButton variant="primary" :icon="Plus" to="/accounting/journal/new">قيد يدوي</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <div class="flex flex-wrap items-center gap-2">
        <SegmentedControl v-model="type" :options="typeOptions" />
        <SearchInput v-model="search" placeholder="رقم القيد، البيان، أو المستند" />
      </div>
      <DateRangeFilter v-model:from="from" v-model:to="to" />
    </div>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="Wallet"
      empty-title="لا توجد قيود في هذه الفترة"
      @retry="reload"
      @row-click="(e) => router.push(`/accounting/journal/${e.id}`)"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
      <template #cell-description="{ row }">
        <span class="line-clamp-1">{{ row.description }}</span>
        <span v-if="row.reversed" class="text-xs text-danger">معكوس</span>
      </template>
      <template #cell-type="{ row }">
        <StatusBadge :tone="row.type === 'SYSTEM' ? 'primary' : 'neutral'" :label="row.type === 'SYSTEM' ? 'آلي' : 'يدوي'" />
      </template>
      <template #cell-lines="{ row }"><span class="num text-text-secondary">{{ formatNumber(row.lines.length) }}</span></template>
      <template #cell-totalDebit="{ row }"><MoneyText :value="row.totalDebit" /></template>
    </DataTable>
  </div>
</template>
