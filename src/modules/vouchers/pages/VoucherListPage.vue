<script setup lang="ts">
import { computed, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Wallet } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import FilterBar from '@/modules/core/components/blocks/FilterBar.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getVouchers } from '../services/voucherService';
import type { Voucher, VoucherKind } from '../types';

const router = useRouter();
const route = useRoute();
const auth = useAuthStore();

const kind = computed(() => (typeof route.query.kind === 'string' ? (route.query.kind as VoucherKind) : undefined));
const search = computed(() => (typeof route.query.q === 'string' ? route.query.q : ''));
const from = computed(() => (typeof route.query.from === 'string' ? route.query.from : ''));
const to = computed(() => (typeof route.query.to === 'string' ? route.query.to : ''));

const { data, loading, error, reload } = useAsync(() =>
  getVouchers({ kind: kind.value, from: from.value || undefined, to: to.value || undefined, search: search.value || undefined }),
);
watch([kind, from, to, search], reload);

const KIND_LABEL: Record<VoucherKind, string> = { RECEIPT: 'قبض عام', PAYMENT: 'صرف عام', TRANSFER: 'تحويل', OWNER: 'مالك' };

const kindOptions = [
  { value: 'RECEIPT', label: 'قبض عام' },
  { value: 'PAYMENT', label: 'صرف عام' },
  { value: 'TRANSFER', label: 'تحويل' },
  { value: 'OWNER', label: 'مالك' },
];

const columns: Column<Voucher>[] = [
  { key: 'number', label: 'الرقم', sortable: true },
  { key: 'date', label: 'التاريخ', type: 'date', sortable: true },
  { key: 'kind', label: 'النوع' },
  { key: 'description', label: 'الوصف' },
  { key: 'amount', label: 'المبلغ', type: 'money', sortable: true },
];
</script>

<template>
  <ListPage
    title="السندات العامة"
    subtitle="قبض/صرف عام، تحويل بين الحسابات، مسحوبات ورأس مال المالك"
    :primary-action-label="auth.can('payments', 'write') ? 'سند جديد' : undefined"
    :primary-action-to="{ name: 'voucher-new' }"
  >
    <template #filters>
      <FilterBar search-placeholder="الرقم أو الوصف" :filters="[{ key: 'kind', label: 'النوع', options: kindOptions }]" date-range />
    </template>

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
      <template #cell-kind="{ row }">{{ KIND_LABEL[row.kind] }}</template>
    </DataTable>
  </ListPage>
</template>
