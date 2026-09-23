<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ReceiptText, ShoppingCart } from '@lucide/vue';
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
import { INVOICE_STATUS, PAYMENT_STATUS, SALE_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getInvoices, type InvoiceRow } from '../services/invoiceService';

type View = 'all' | 'PAID' | 'PARTIALLY_PAID' | 'UNPAID' | 'open' | 'REFUNDED';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();

const openOnly = route.query.payment === 'open';
const view = ref<View>(openOnly ? 'open' : 'all');
const search = ref(String(route.query.q ?? ''));
// Unpaid invoices can be old — show all dates when arriving from the dashboard's "unpaid" card.
const from = ref(openOnly ? '' : daysAgoKey(6));
const to = ref(openOnly ? '' : todayKey());

const { data, loading, error, reload } = useAsync(() => getInvoices({ from: from.value || undefined, to: to.value || undefined }));
watch([from, to], reload);
watch(view, (v) => router.replace({ query: { payment: v === 'open' ? 'open' : undefined } }));

function matches(r: InvoiceRow, v: View) {
  if (v === 'all') return true;
  if (v === 'REFUNDED') return r.status === 'REFUNDED';
  if (v === 'open') return r.status === 'COMPLETED' && r.outstanding > 0;
  return r.status !== 'REFUNDED' && r.paymentStatus === v;
}

const rows = computed(() => {
  const q = search.value.trim().toLowerCase();
  return (data.value ?? []).filter((r) => matches(r, view.value) && (!q || `${r.number} ${r.customerName ?? ''}`.toLowerCase().includes(q)));
});

const viewOptions = computed(() =>
  (
    [
      ['all', 'الكل'],
      ['PAID', 'مدفوعة'],
      ['PARTIALLY_PAID', 'جزئياً'],
      ['UNPAID', 'غير مدفوعة'],
      ['open', 'مستحقات مفتوحة'],
      ['REFUNDED', 'مسترجعة'],
    ] as [View, string][]
  ).map(([value, label]) => ({ value, label, count: data.value?.filter((r) => matches(r, value)).length })),
);

const summary = computed(() => ({
  total: rows.value.reduce((a, r) => a + r.grandTotal - r.refundedAmount, 0),
  outstanding: rows.value.reduce((a, r) => a + r.outstanding, 0),
}));

const columns: Column<InvoiceRow>[] = [
  { key: 'number', label: 'رقم الفاتورة', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'customerName', label: 'العميل', sortable: true, sortValue: (r) => r.customerName ?? '' },
  { key: 'paymentMethod', label: 'الدفع' },
  { key: 'status', label: 'الحالة' },
  { key: 'grandTotal', label: 'الإجمالي', numeric: true, sortable: true },
  { key: 'outstanding', label: 'المتبقي', numeric: true, sortable: true },
];
</script>

<template>
  <div>
    <PageHeader title="الفواتير" subtitle="فواتير المبيعات وحالة السداد والمرتجعات">
      <template v-if="auth.can('pos', 'write')" #actions>
        <AppButton variant="primary" :icon="ShoppingCart" to="/pos">بيع جديد</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <SegmentedControl v-model="view" :options="viewOptions" />
      <div class="flex flex-wrap items-center gap-2">
        <SearchInput v-model="search" placeholder="رقم الفاتورة أو اسم العميل" />
      </div>
    </div>
    <div class="mb-3"><DateRangeFilter v-model:from="from" v-model:to="to" /></div>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="ReceiptText"
      empty-title="لا توجد فواتير مطابقة"
      empty-description="غيّر الفترة أو الفلتر لعرض فواتير أخرى"
      @retry="reload"
      @row-click="(r) => router.push(`/invoices/${r.id}`)"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
      <template #cell-customerName="{ row }">
        <span :class="!row.customerName && 'text-text-secondary'">{{ row.customerName ?? 'عميل نقدي' }}</span>
      </template>
      <template #cell-paymentMethod="{ row }"><span class="text-text-secondary">{{ SALE_METHOD_LABEL[row.paymentMethod] }}</span></template>
      <template #cell-status="{ row }">
        <StatusBadge v-if="row.status === 'REFUNDED'" :tone="INVOICE_STATUS.REFUNDED.tone" :label="INVOICE_STATUS.REFUNDED.label" />
        <StatusBadge v-else :tone="PAYMENT_STATUS[row.paymentStatus].tone" :label="PAYMENT_STATUS[row.paymentStatus].label" />
        <span v-if="row.status !== 'REFUNDED' && row.refundedAmount > 0" class="ms-1.5 text-tiny text-danger">مرتجع جزئي</span>
      </template>
      <template #cell-grandTotal="{ row }"><MoneyText :value="row.grandTotal" /></template>
      <template #cell-outstanding="{ row }"><MoneyText :value="row.outstanding" dash-zero :class="row.outstanding > 0 && 'text-warning'" /></template>
    </DataTable>

    <p v-if="rows.length" class="mt-3 text-xs text-text-secondary">
      {{ formatNumber(rows.length) }} فاتورة · الصافي بعد المرتجعات <MoneyText :value="summary.total" class="text-text-primary" /> · المتبقي
      <MoneyText :value="summary.outstanding" class="text-text-primary" />
    </p>
  </div>
</template>
