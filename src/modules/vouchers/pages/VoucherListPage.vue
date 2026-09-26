<script setup lang="ts">
import { ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { Plus, Wallet } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getVouchers } from '../services/voucherService';
import type { Voucher, VoucherKind } from '../types';

const router = useRouter();
const auth = useAuthStore();
const kind = ref<VoucherKind | 'all'>('all');
const search = ref('');
const from = ref('');
const to = ref('');

const { data, loading, error, reload } = useAsync(() => getVouchers({ kind: kind.value === 'all' ? undefined : kind.value, from: from.value || undefined, to: to.value || undefined, search: search.value || undefined }));
watch([kind, from, to, search], reload);

const KIND_LABEL: Record<VoucherKind, string> = { RECEIPT: 'قبض عام', PAYMENT: 'صرف عام', TRANSFER: 'تحويل', OWNER: 'مالك' };

const columns: Column<Voucher>[] = [
  { key: 'number', label: 'الرقم', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'kind', label: 'النوع' },
  { key: 'description', label: 'الوصف' },
  { key: 'amount', label: 'المبلغ', numeric: true, sortable: true },
];
</script>

<template>
  <div>
    <PageHeader title="السندات العامة" subtitle="قبض/صرف عام، تحويل بين الحسابات، مسحوبات ورأس مال المالك">
      <template v-if="auth.can('payments', 'write')" #actions>
        <AppButton variant="primary" :icon="Plus" :to="{ name: 'voucher-new' }">سند جديد</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <SegmentedControl
        v-model="kind"
        :options="[
          { value: 'all', label: 'الكل' },
          { value: 'RECEIPT', label: 'قبض عام' },
          { value: 'PAYMENT', label: 'صرف عام' },
          { value: 'TRANSFER', label: 'تحويل' },
          { value: 'OWNER', label: 'مالك' },
        ]"
      />
      <SearchInput v-model="search" placeholder="الرقم أو الوصف" />
    </div>
    <div class="mb-3"><DateRangeFilter v-model:from="from" v-model:to="to" /></div>

    <DataTable
      :columns="columns"
      :rows="data ?? []"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="Wallet"
      empty-title="لا توجد سندات"
      @retry="reload"
      @row-click="(r) => router.push({ name: 'voucher-detail', params: { id: r.id } })"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDate(row.date) }}</span></template>
      <template #cell-kind="{ row }">{{ KIND_LABEL[row.kind] }}</template>
      <template #cell-amount="{ row }"><MoneyText :value="row.amount" /></template>
    </DataTable>
  </div>
</template>
