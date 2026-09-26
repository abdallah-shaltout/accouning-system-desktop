<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { BookOpen, HandCoins, Printer, Undo2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import DetailPage, { type DetailTab } from '@/modules/core/components/layouts/DetailPage.vue';
import type { MetaChip } from '@/modules/core/components/blocks/DetailHeader.vue';
import { isTauri } from '@tauri-apps/api/core';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { INVOICE_STATUS, PAYMENT_METHOD_LABEL, PAYMENT_STATUS, SALE_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { renderAndSave } from '@/modules/core/services/pdfService';
import { useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useRouter } from 'vue-router';
import { round2 } from '../helpers/totals';
import { getInvoice, type InvoiceDetail } from '../services/invoiceService';

const route = useRoute('invoice');
const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
const id = String(route.params.id);
const { data, error, reload } = useAsync(() => getInvoice(id));
const inv = computed(() => data.value);

const canRefund = computed(() => auth.can('sales', 'write') && inv.value?.status === 'COMPLETED');
const canPay = computed(() => auth.can('payments', 'write') && !!inv.value?.customerId && (inv.value?.outstanding ?? 0) > 0);

const status = computed(() => {
  if (!inv.value) return undefined;
  return inv.value.status === 'REFUNDED' ? INVOICE_STATUS.REFUNDED : PAYMENT_STATUS[inv.value.paymentStatus];
});

const chips = computed<MetaChip[]>(() => {
  if (!inv.value) return [];
  return [
    { label: 'التاريخ', value: formatDateTime(inv.value.date) },
    { label: 'الكاشير', value: inv.value.cashierName },
  ];
});

const tabs = computed<DetailTab[]>(() => {
  const t: DetailTab[] = [{ key: 'lines', label: 'الأصناف' }];
  if (inv.value?.refunds.length) t.push({ key: 'refunds', label: 'المرتجعات' });
  if (inv.value?.payments.length) t.push({ key: 'payments', label: 'الدفعات اللاحقة' });
  return t;
});

const lineColumns: Column<InvoiceDetail['lines'][number]>[] = [
  { key: 'name', label: 'الصنف' },
  { key: 'qty', label: 'الكمية', type: 'number' },
  { key: 'price', label: 'السعر', type: 'money' },
  { key: 'returned', label: 'المرتجع', type: 'number' },
  { key: 'total', label: 'الإجمالي', type: 'money' },
];

function lineTotal(l: InvoiceDetail['lines'][number]): number {
  return round2(l.qty * l.price - l.discount);
}

/**
 * Phase 11a integration point (docs/v2/12-documents-pdf-excel.md §2): in the
 * desktop app, render a real PDF via the Rust engine, save it and open it.
 * In browser dev mode `renderAndSave` itself shows the "available in the
 * desktop app" toast and returns false, so we fall back to the v1 print
 * route exactly as before.
 */
async function print() {
  if (isTauri()) {
    const ok = await renderAndSave('invoice', id, `${inv.value?.number ?? id}.pdf`);
    if (ok) return;
    toast.error('تعذر إنشاء ملف PDF');
    return;
  }
  router.push({ name: 'invoice-print', params: { id } });
}
</script>

<template>
  <div>
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <DetailPage
      v-else
      :title="inv ? `فاتورة ${inv.number}` : '…'"
      :status="status"
      :chips="chips"
      :back="{ name: 'invoices' }"
      :tabs="tabs"
    >
      <template #actions>
        <AppButton v-if="canRefund" :icon="Undo2" :to="{ name: 'invoice-refund', params: { id } }">إرجاع</AppButton>
        <AppButton v-if="canPay" :icon="HandCoins" :to="{ name: 'payment-new', query: { type: 'RECEIVED', party: inv?.customerId, ref: id } }">تسجيل دفعة</AppButton>
        <AppButton variant="primary" :icon="Printer" @click="print">طباعة</AppButton>
      </template>

      <template #tab-lines>
        <AppCard padding="none">
          <div v-if="!inv" class="p-4"><SkeletonBlock :lines="6" /></div>
          <DataTable v-else :columns="lineColumns" :rows="inv.lines" row-key="id">
            <template #cell-name="{ row }">
              <RouterLink :to="{ name: 'product', params: { id: row.productId } }" class="hover:text-primary">{{ row.name }}</RouterLink>
            </template>
            <template #cell-returned="{ row }">
              <span v-if="inv!.returnedQty[row.id]" class="num text-danger">{{ formatNumber(inv!.returnedQty[row.id]) }}</span>
              <span v-else class="text-text-secondary">—</span>
            </template>
            <template #cell-total="{ row }">
              <MoneyText :value="lineTotal(row)" />
            </template>
          </DataTable>
        </AppCard>
      </template>

      <template v-if="inv?.refunds.length" #tab-refunds>
        <AppCard padding="none">
          <ul class="divide-y divide-border text-body">
            <li v-for="r in inv.refunds" :key="r.id" class="flex items-center justify-between gap-3 px-4 py-2.5">
              <div>
                <span class="num font-medium">{{ r.number }}</span>
                <span class="ms-2 text-text-secondary">{{ r.reason ?? 'بدون سبب' }}</span>
                <span class="num block text-xs text-text-secondary">{{ formatDateTime(r.date) }}</span>
              </div>
              <div class="text-end">
                <MoneyText :value="r.grandTotal" class="text-danger" />
                <span class="block text-tiny text-text-secondary">
                  <template v-if="r.cashBack">مسترد نقداً <MoneyText :value="r.cashBack" plain /></template>
                  <template v-if="r.cashBack && r.settledToReceivable"> · </template>
                  <template v-if="r.settledToReceivable">خصم من الحساب <MoneyText :value="r.settledToReceivable" plain /></template>
                </span>
              </div>
            </li>
          </ul>
        </AppCard>
      </template>

      <template v-if="inv?.payments.length" #tab-payments>
        <AppCard padding="none">
          <ul class="divide-y divide-border text-body">
            <li v-for="p in inv.payments" :key="p.id" class="flex items-center justify-between px-4 py-2.5">
              <div>
                <span class="num font-medium">{{ p.number }}</span>
                <span class="ms-2 text-text-secondary">{{ PAYMENT_METHOD_LABEL[p.method] }}</span>
                <span class="num block text-xs text-text-secondary">{{ formatDateTime(p.date) }}</span>
              </div>
              <MoneyText :value="p.amount" class="text-success" />
            </li>
          </ul>
        </AppCard>
      </template>

      <template #aside>
          <AppCard title="الملخص" padding="sm">
            <SkeletonBlock v-if="!inv" :lines="5" />
            <dl v-else class="space-y-1.5 text-body">
              <!-- v2 phase 9 (docs/v2/10 §2): an FC invoice's totals are in ITS OWN currency; a
                   base-currency equivalent line (at the invoice's own rate) is shown alongside the
                   grand total, mirroring the doc's "VAT-in-SAR" requirement for the total itself. -->
              <div v-if="inv.currency" class="mb-1 rounded-md bg-primary/10 px-2 py-1 text-tiny text-primary">
                فاتورة بعملة {{ inv.currency }} — سعر الصرف <span class="num">{{ inv.exchangeRate }}</span>
              </div>
              <div class="flex justify-between"><dt class="text-text-secondary">المجموع</dt><dd><MoneyText :value="inv.subTotal" :currency="inv.currency" /></dd></div>
              <div v-if="inv.discountAmount" class="flex justify-between">
                <dt class="text-text-secondary">الخصم <span class="num">({{ formatNumber(inv.discountRate) }}%)</span></dt>
                <dd>−<MoneyText :value="inv.discountAmount" :currency="inv.currency" /></dd>
              </div>
              <div class="flex justify-between">
                <dt class="text-text-secondary">الضريبة <span class="num">({{ formatNumber(inv.taxRate) }}%)</span></dt>
                <dd><MoneyText :value="inv.taxAmount" :currency="inv.currency" /></dd>
              </div>
              <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>الإجمالي</dt><dd><MoneyText :value="inv.grandTotal" :currency="inv.currency" /></dd></div>
              <div v-if="inv.currency && inv.exchangeRate" class="flex justify-between text-tiny text-text-secondary">
                <dt>ما يعادل بالعملة الأساسية</dt><dd><MoneyText :value="inv.grandTotal * inv.exchangeRate" /></dd>
              </div>
              <div v-if="inv.refundedAmount" class="flex justify-between text-danger"><dt>المرتجع</dt><dd>−<MoneyText :value="inv.refundedAmount" :currency="inv.currency" /></dd></div>
              <div class="flex justify-between"><dt class="text-text-secondary">المدفوع</dt><dd><MoneyText :value="inv.paidAmount" :currency="inv.currency" /></dd></div>
              <div class="flex justify-between font-medium" :class="inv.outstanding > 0 ? 'text-warning' : ''">
                <dt>المتبقي</dt><dd><MoneyText :value="inv.outstanding" :currency="inv.currency" /></dd>
              </div>
            </dl>
          </AppCard>

          <AppCard title="العميل والدفع" padding="sm">
            <SkeletonBlock v-if="!inv" :lines="3" />
            <dl v-else class="space-y-1.5 text-body">
              <div class="flex justify-between gap-3">
                <dt class="text-text-secondary">العميل</dt>
                <dd>
                  <RouterLink v-if="inv.customer" :to="{ name: 'customer', params: { id: inv.customer.id } }" class="text-primary hover:underline">{{ inv.customer.name }}</RouterLink>
                  <span v-else>عميل نقدي</span>
                </dd>
              </div>
              <div class="flex justify-between"><dt class="text-text-secondary">طريقة الدفع</dt><dd>{{ SALE_METHOD_LABEL[inv.paymentMethod] }}</dd></div>
              <div v-if="inv.tenderedAmount" class="flex justify-between"><dt class="text-text-secondary">المستلم</dt><dd><MoneyText :value="inv.tenderedAmount" /></dd></div>
              <div v-if="inv.note" class="border-t border-border pt-1.5 text-text-secondary">{{ inv.note }}</div>
            </dl>
          </AppCard>

          <AppCard v-if="inv && auth.can('accounting')" title="القيود المحاسبية" padding="none">
            <ul class="divide-y divide-border text-body">
              <li v-for="e in inv.journalEntries" :key="e.id">
                <RouterLink :to="{ name: 'journal-entry', params: { id: e.id } }" class="flex items-center gap-2 px-4 py-2 hover:bg-surface-hover">
                  <BookOpen class="size-3.5 shrink-0 text-text-secondary" />
                  <span class="num text-primary">{{ e.number }}</span>
                  <span class="truncate text-xs text-text-secondary">{{ e.description }}</span>
                </RouterLink>
              </li>
            </ul>
          </AppCard>
      </template>
    </DetailPage>
  </div>
</template>
