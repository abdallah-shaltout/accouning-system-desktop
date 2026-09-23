<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { HandCoins, Plus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { daysAgoKey, formatDateTime, todayKey } from '@/modules/core/helpers/format';
import { PAYMENT_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { matchesSearch } from '@/modules/core/helpers/search';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getPayments, type PaymentRow } from '../services/paymentService';
import type { AllocationStatus, PaymentMethod, PaymentType } from '../types';

const ALLOCATION_LABEL: Record<AllocationStatus, string> = { full: 'مخصص بالكامل', partial: 'مخصص جزئياً', unallocated: 'غير مخصص' };
const ALLOCATION_TONE: Record<AllocationStatus, 'success' | 'warning' | 'neutral'> = { full: 'success', partial: 'warning', unallocated: 'neutral' };

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();

const highlight = typeof route.query.highlight === 'string' ? route.query.highlight : undefined;
const type = ref<PaymentType | 'all'>('all');
const method = ref<PaymentMethod | ''>('');
const search = ref('');
// A highlighted payment may be older than 30 days — widen the range so it's visible.
const from = ref(highlight ? '' : daysAgoKey(29));
const to = ref(highlight ? '' : todayKey());

const { data, loading, error, reload } = useAsync(() =>
  getPayments({ method: method.value || undefined, from: from.value || undefined, to: to.value || undefined }),
);
watch([method, from, to], reload);

const rows = computed(() =>
  (data.value ?? []).filter(
    (p) => (type.value === 'all' || p.type === type.value) && matchesSearch([p.number, p.partyName, p.targetRefNumber], search.value),
  ),
);

const totals = computed(() => {
  const list = data.value ?? [];
  const received = list.filter((p) => p.type === 'RECEIVED').reduce((a, p) => a + p.amount, 0);
  const paid = list.filter((p) => p.type === 'PAID').reduce((a, p) => a + p.amount, 0);
  return { received, paid, net: received - paid };
});

const typeOptions = computed(() => [
  { value: 'all' as const, label: 'الكل', count: data.value?.length },
  { value: 'RECEIVED' as const, label: 'سندات قبض', count: data.value?.filter((p) => p.type === 'RECEIVED').length },
  { value: 'PAID' as const, label: 'سندات صرف', count: data.value?.filter((p) => p.type === 'PAID').length },
]);

const columns: Column<PaymentRow>[] = [
  { key: 'number', label: 'رقم السند', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'type', label: 'النوع' },
  { key: 'partyName', label: 'الطرف', sortable: true },
  { key: 'allocationStatus', label: 'التخصيص' },
  { key: 'method', label: 'الطريقة' },
  { key: 'amount', label: 'المبلغ', numeric: true, sortable: true },
];
</script>

<template>
  <div>
    <PageHeader title="سندات القبض والصرف" subtitle="تحصيل مستحقات العملاء وسداد الموردين — يمكن للسند الواحد تسوية عدة مستندات، والباقي يظهر رصيداً على الطرف">
      <template v-if="auth.can('payments', 'write')" #actions>
        <AppButton :icon="Plus" :to="{ path: '/payments/new', query: { type: 'PAID' } }">سند صرف</AppButton>
        <AppButton variant="primary" :icon="Plus" :to="{ path: '/payments/new', query: { type: 'RECEIVED' } }">سند قبض</AppButton>
      </template>
    </PageHeader>

    <div class="mb-4 grid gap-4 sm:grid-cols-3">
      <AppCard padding="sm">
        <p class="text-xs text-text-secondary">المقبوضات في الفترة</p>
        <p class="mt-1 text-lg font-semibold text-success"><MoneyText :value="totals.received" /></p>
      </AppCard>
      <AppCard padding="sm">
        <p class="text-xs text-text-secondary">المدفوعات في الفترة</p>
        <p class="mt-1 text-lg font-semibold"><MoneyText :value="totals.paid" /></p>
      </AppCard>
      <AppCard padding="sm">
        <p class="text-xs text-text-secondary">صافي التدفق</p>
        <p class="mt-1 text-lg font-semibold"><MoneyText :value="totals.net" signed /></p>
      </AppCard>
    </div>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <div class="flex flex-wrap items-center gap-2">
        <SegmentedControl v-model="type" :options="typeOptions" />
        <AppSelect
          v-model="method"
          class="w-36"
          placeholder="كل الطرق"
          :options="(Object.keys(PAYMENT_METHOD_LABEL) as PaymentMethod[]).map((m) => ({ value: m, label: PAYMENT_METHOD_LABEL[m] }))"
        />
      </div>
      <SearchInput v-model="search" placeholder="رقم السند، الطرف، أو المستند" />
    </div>
    <div class="mb-3"><DateRangeFilter v-model:from="from" v-model:to="to" /></div>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      :highlight-key="highlight"
      :empty-icon="HandCoins"
      empty-title="لا توجد سندات في هذه الفترة"
      clickable
      @retry="reload"
      @row-click="(p) => router.push(`/payments/${p.id}`)"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
      <template #cell-type="{ row }">
        <StatusBadge :tone="row.type === 'RECEIVED' ? 'success' : 'neutral'" :label="row.type === 'RECEIVED' ? 'قبض' : 'صرف'" />
      </template>
      <template #cell-partyName="{ row }">
        <RouterLink :to="`/${row.targetType === 'customer' ? 'customers' : 'suppliers'}/${row.targetId}`" class="hover:text-primary" @click.stop>{{ row.partyName }}</RouterLink>
      </template>
      <template #cell-allocationStatus="{ row }">
        <StatusBadge :tone="ALLOCATION_TONE[row.allocationStatus]" :label="ALLOCATION_LABEL[row.allocationStatus]" />
      </template>
      <template #cell-method="{ row }"><span class="text-text-secondary">{{ PAYMENT_METHOD_LABEL[row.method] }}</span></template>
      <template #cell-amount="{ row }">
        <MoneyText :value="row.amount" :class="row.type === 'RECEIVED' ? 'text-success' : ''" />
      </template>
    </DataTable>
  </div>
</template>
