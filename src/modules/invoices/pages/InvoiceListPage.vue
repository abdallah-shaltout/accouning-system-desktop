<script setup lang="ts">
/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §6 "Invoice list v2"): filters (branch/source/type/
 * payment-status/overdue/customer/cashier/date/amount), saved views, footer totals, bulk actions
 * (Excel export via DataTable's built-in exporter, PDF .zip download, print). Branch filtering is
 * inert (single branch until Phase 9, same as every other v2 screen) — the field exists in the
 * shape for when Phase 9 lands.
 */
import { computed, ref, watch } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { useRoute, useRouter } from 'vue-router';
import { Bookmark, Download, Printer, ReceiptText, ShoppingCart } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useToast } from '@/modules/core/controllers/useToast';
import { daysAgoKey, formatDate, formatDateTime, formatNumber, todayKey } from '@/modules/core/helpers/format';
import { INVOICE_STATUS, PAYMENT_STATUS, SALE_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { matchesSearch } from '@/modules/core/helpers/search';
import { getCustomers } from '@/modules/parties/services/partyService';
import ReportPrintDialog from '@/modules/reports/components/ReportPrintDialog.vue';
import { count, kpis, money, row, table as printTable } from '@/modules/reports/print/build';
import { useOfficialPrint } from '@/modules/reports/print/useOfficialPrint';
import { getUsers } from '@/modules/users/services/userService';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getInvoices, isOverdue, type InvoiceRow } from '../services/invoiceService';

type View = 'all' | 'PAID' | 'PARTIALLY_PAID' | 'UNPAID' | 'open' | 'REFUNDED' | 'overdue' | 'today-branch';
type SavedView = { id: View; label: string };

const SAVED_VIEWS: SavedView[] = [
  { id: 'overdue', label: 'آجل متأخر' },
  { id: 'today-branch', label: 'اليوم — فرعي' },
];

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const toast = useToast();

const openOnly = route.query.payment === 'open';
const view = ref<View>(openOnly ? 'open' : 'all');
const search = ref(String(route.query.q ?? ''));
const from = ref(openOnly ? '' : daysAgoKey(6));
const to = ref(openOnly ? '' : todayKey());
const source = ref<'' | 'POS' | 'DESK'>('');
const cashierId = ref('');
const customerId = ref('');
const minAmount = ref<number | undefined>();
const maxAmount = ref<number | undefined>();

const { data, loading, error, reload } = useAsync(() => getInvoices({ from: from.value || undefined, to: to.value || undefined }));
watch([from, to], reload);

const { data: cashiers } = useAsync(() => getUsers());
const { data: customers } = useAsync(() => getCustomers());

watch(view, (v) => {
  router.replace({ query: { payment: v === 'open' ? 'open' : undefined } });
  if (v === 'overdue') {
    from.value = '';
    to.value = '';
  } else if (v === 'today-branch') {
    from.value = todayKey();
    to.value = todayKey();
  }
});

function matches(r: InvoiceRow, v: View) {
  if (v === 'all') return true;
  if (v === 'REFUNDED') return r.status === 'REFUNDED';
  if (v === 'open') return r.status === 'COMPLETED' && r.outstanding > 0;
  if (v === 'overdue') return isOverdue(r);
  if (v === 'today-branch') return new Date(r.date).toDateString() === new Date().toDateString();
  return r.status !== 'REFUNDED' && r.paymentStatus === v;
}

const rows = computed(() =>
  (data.value ?? [])
    .filter((r) => matches(r, view.value))
    .filter((r) => matchesSearch([r.number, r.customerName], search.value))
    .filter((r) => !source.value || (r.source ?? 'POS') === source.value)
    .filter((r) => !cashierId.value || r.cashierId === cashierId.value)
    .filter((r) => !customerId.value || r.customerId === customerId.value)
    .filter((r) => minAmount.value === undefined || r.grandTotal >= minAmount.value)
    .filter((r) => maxAmount.value === undefined || r.grandTotal <= maxAmount.value),
);

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
  count: rows.value.length,
}));

const columns: Column<InvoiceRow>[] = [
  { key: 'number', label: 'رقم الفاتورة', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'customerName', label: 'العميل', sortable: true, sortValue: (r) => r.customerName ?? '' },
  { key: 'source', label: 'المصدر' },
  { key: 'paymentMethod', label: 'الدفع' },
  { key: 'status', label: 'الحالة' },
  { key: 'grandTotal', label: 'الإجمالي', numeric: true, sortable: true },
  { key: 'outstanding', label: 'المتبقي', numeric: true, sortable: true },
];

// --- Bulk actions (docs/v2/06 §6 "Bulk actions: export Excel, download PDFs as a .zip, print") ---
const zipping = ref(false);
async function downloadPdfsZip() {
  if (!isTauri()) {
    toast.info('متاح فقط في تطبيق سطح المكتب');
    return;
  }
  if (!rows.value.length) return;
  zipping.value = true;
  try {
    const pdfService = await import('@/modules/core/services/pdfService');
    const { zipSync } = await import('fflate');
    const files: Record<string, Uint8Array> = {};
    for (const r of rows.value) {
      const outcome = await pdfService.render('invoice', r.id);
      if (outcome.ok && outcome.pdfBytes) files[`${r.number}.pdf`] = outcome.pdfBytes;
    }
    if (!Object.keys(files).length) {
      toast.warning('تعذر إنشاء أي ملف PDF');
      return;
    }
    const zipped = zipSync(files);
    const [{ save }, { writeFile }] = await Promise.all([import('@tauri-apps/plugin-dialog'), import('@tauri-apps/plugin-fs')]);
    const path = await save({ defaultPath: `invoices-${todayKey()}.zip`, filters: [{ name: 'Zip', extensions: ['zip'] }] });
    if (!path) return;
    await writeFile(path, zipped);
    toast.success('تم تنزيل الملف المضغوط', `${Object.keys(files).length} فاتورة`);
  } catch (err) {
    toast.error(err, 'تعذر إنشاء الملف المضغوط');
  } finally {
    zipping.value = false;
  }
}

// Print = an official invoice register of the rows currently listed (letterhead, filters, totals),
// rendered as its own document — never a print of this screen.
const { open: printOpen, doc: printDoc, show: showPrint } = useOfficialPrint();

function statusText(r: InvoiceRow): string {
  const base = r.status === 'REFUNDED' ? INVOICE_STATUS.REFUNDED.label : PAYMENT_STATUS[r.paymentStatus].label;
  return [base, r.status !== 'REFUNDED' && r.refundedAmount > 0 ? 'مرتجع جزئي' : '', isOverdue(r) ? 'متأخر' : ''].filter(Boolean).join(' · ');
}

function printAll() {
  const list = rows.value;
  const filters = [
    view.value !== 'all' ? (viewOptions.value.find((o) => o.value === view.value)?.label ?? SAVED_VIEWS.find((v) => v.id === view.value)?.label) : '',
    source.value ? (source.value === 'POS' ? 'نقطة بيع' : 'فاتورة مكتبية') : '',
    cashierId.value ? `الكاشير: ${cashiers.value?.find((u) => u.id === cashierId.value)?.name ?? ''}` : '',
    customerId.value ? `العميل: ${customers.value?.find((c) => c.id === customerId.value)?.name ?? ''}` : '',
    search.value ? `بحث: ${search.value}` : '',
  ].filter(Boolean);
  showPrint({
    title: 'سجل فواتير المبيعات',
    subtitle: 'الفواتير المعروضة حسب عوامل التصفية الحالية',
    leadMeta: [
      { label: 'الفترة من', value: from.value ? formatDate(from.value) : 'البداية' },
      { label: 'الفترة إلى', value: to.value ? formatDate(to.value) : formatDate(new Date().toISOString()) },
      ...(filters.length ? [{ label: 'التصفية', value: filters.join(' — ') }] : []),
    ],
    blocks: [
      kpis([
        { label: 'عدد الفواتير', value: count(summary.value.count) },
        { label: 'إجمالي الفواتير', value: money(list.reduce((a, r) => a + r.grandTotal, 0)) },
        { label: 'المرتجعات', value: money(list.reduce((a, r) => a + r.refundedAmount, 0)) },
        { label: 'الصافي بعد المرتجعات', value: money(summary.value.total), emphasis: true },
        { label: 'المتبقي على العملاء', value: money(summary.value.outstanding) },
      ]),
      printTable(
        [
          { label: '#', dim: true, align: 'center', width: 0.4 },
          { label: 'رقم الفاتورة', width: 1.2 },
          { label: 'التاريخ', dim: true, width: 1.3 },
          { label: 'العميل', width: 2 },
          { label: 'المصدر', width: 0.9 },
          { label: 'الدفع', width: 0.9 },
          { label: 'الحالة', width: 1.3 },
          { label: 'الإجمالي', numeric: true, width: 1.1 },
          { label: 'المرتجع', numeric: true, width: 1 },
          { label: 'المتبقي', numeric: true, width: 1 },
        ],
        [
          ...list.map((r, i) =>
            row([
              count(i + 1),
              r.number,
              formatDateTime(r.date),
              r.customerName ?? 'عميل نقدي',
              (r.source ?? 'POS') === 'POS' ? 'نقطة بيع' : 'مكتبية',
              SALE_METHOD_LABEL[r.paymentMethod],
              statusText(r),
              money(r.grandTotal),
              money(r.refundedAmount, { dashZero: true }),
              money(r.outstanding, { dashZero: true }),
            ]),
          ),
          row(['', '', '', 'الإجمالي', '', '', '', money(list.reduce((a, r) => a + r.grandTotal, 0)), money(list.reduce((a, r) => a + r.refundedAmount, 0)), money(summary.value.outstanding)], 'total'),
        ],
        'لا توجد فواتير مطابقة',
      ),
    ],
  });
}
</script>

<template>
  <div>
    <PageHeader title="الفواتير" subtitle="فواتير المبيعات وحالة السداد والمرتجعات">
      <template #actions>
        <AppButton :icon="Download" :loading="zipping" @click="downloadPdfsZip">تنزيل PDF (مضغوط)</AppButton>
        <AppButton :icon="Printer" :disabled="loading && !data" data-testid="invoices-print" @click="printAll">طباعة</AppButton>
        <AppButton v-if="auth.can('pos', 'write')" variant="primary" :icon="ShoppingCart" :to="{ name: 'pos' }">بيع جديد</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <SegmentedControl v-model="view" :options="viewOptions" />
      <div class="flex flex-wrap items-center gap-2">
        <span v-for="v in SAVED_VIEWS" :key="v.id" class="inline-flex">
          <button
            type="button"
            class="flex h-8 items-center gap-1.5 rounded-full border px-3 text-body transition-colors"
            :class="view === v.id ? 'border-primary bg-primary text-on-primary' : 'border-border text-text-secondary hover:bg-surface-hover'"
            @click="view = v.id"
          >
            <Bookmark class="size-3" />{{ v.label }}
          </button>
        </span>
        <SearchInput v-model="search" placeholder="رقم الفاتورة أو اسم العميل" />
      </div>
    </div>

    <div class="mb-3 grid grid-cols-2 gap-2 md:grid-cols-6">
      <DateRangeFilter v-model:from="from" v-model:to="to" class="col-span-2" />
      <AppSelect v-model="source" :options="[{ value: '', label: 'كل المصادر' }, { value: 'POS', label: 'نقطة بيع' }, { value: 'DESK', label: 'فاتورة مكتبية' }]" />
      <AppSelect v-model="cashierId" :options="[{ value: '', label: 'كل الكاشيرين' }, ...(cashiers ?? []).map((u) => ({ value: u.id, label: u.name }))]" />
      <AppSelect v-model="customerId" :options="[{ value: '', label: 'كل العملاء' }, ...(customers ?? []).map((c) => ({ value: c.id, label: c.name }))]" />
      <div class="flex gap-1">
        <input v-model.number="minAmount" type="number" class="control h-9 w-full" placeholder="من مبلغ" />
        <input v-model.number="maxAmount" type="number" class="control h-9 w-full" placeholder="إلى مبلغ" />
      </div>
    </div>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="ReceiptText"
      empty-title="لا توجد فواتير مطابقة"
      empty-description="غيّر الفترة أو الفلتر لعرض فواتير أخرى"
      export-file-name="invoices"
      @retry="reload"
      @row-click="(r) => router.push({ name: 'invoice', params: { id: r.id } })"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
      <template #cell-customerName="{ row }">
        <span :class="!row.customerName && 'text-text-secondary'">{{ row.customerName ?? 'عميل نقدي' }}</span>
      </template>
      <template #cell-source="{ row }"><span class="text-xs text-text-secondary">{{ (row.source ?? 'POS') === 'POS' ? 'نقطة بيع' : 'مكتبية' }}</span></template>
      <template #cell-paymentMethod="{ row }"><span class="text-text-secondary">{{ SALE_METHOD_LABEL[row.paymentMethod] }}</span></template>
      <template #cell-status="{ row }">
        <StatusBadge v-if="row.status === 'REFUNDED'" :tone="INVOICE_STATUS.REFUNDED.tone" :label="INVOICE_STATUS.REFUNDED.label" />
        <StatusBadge v-else :tone="PAYMENT_STATUS[row.paymentStatus].tone" :label="PAYMENT_STATUS[row.paymentStatus].label" />
        <span v-if="row.status !== 'REFUNDED' && row.refundedAmount > 0" class="ms-1.5 text-tiny text-danger">مرتجع جزئي</span>
        <span v-if="isOverdue(row)" class="ms-1.5 text-tiny text-danger">متأخر</span>
      </template>
      <template #cell-grandTotal="{ row }"><MoneyText :value="row.grandTotal" /></template>
      <template #cell-outstanding="{ row }"><MoneyText :value="row.outstanding" dash-zero :class="row.outstanding > 0 && 'text-warning'" /></template>
      <template #footer>
        <div class="flex items-center justify-between border-t border-border px-4 py-2.5 text-body">
          <span class="text-text-secondary">{{ formatNumber(summary.count) }} فاتورة</span>
          <span class="flex gap-4">
            <span>الصافي بعد المرتجعات <MoneyText :value="summary.total" class="text-text-primary" /></span>
            <span>المتبقي <MoneyText :value="summary.outstanding" class="text-text-primary" /></span>
          </span>
        </div>
      </template>
    </DataTable>

    <ReportPrintDialog v-model:open="printOpen" :doc="printDoc" :file-name="`سجل الفواتير ${from || ''}_${to || ''}`" />
  </div>
</template>
