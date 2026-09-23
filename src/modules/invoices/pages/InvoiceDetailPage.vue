<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { BookOpen, HandCoins, Printer, Undo2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { isTauri } from '@tauri-apps/api/core';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { INVOICE_STATUS, PAYMENT_METHOD_LABEL, PAYMENT_STATUS, SALE_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { renderAndSave } from '@/modules/core/services/pdfService';
import { useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useRouter } from 'vue-router';
import { round2 } from '../helpers/totals';
import { getInvoice } from '../services/invoiceService';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
const id = String(route.params.id);
const { data, error, reload } = useAsync(() => getInvoice(id));
const inv = computed(() => data.value);

const canRefund = computed(() => auth.can('sales', 'write') && inv.value?.status === 'COMPLETED');
const canPay = computed(() => auth.can('payments', 'write') && !!inv.value?.customerId && (inv.value?.outstanding ?? 0) > 0);

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
  router.push(`/print/invoices/${id}`);
}
</script>

<template>
  <div>
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <template v-else>
      <PageHeader :title="inv ? `فاتورة ${inv.number}` : '…'" back="/invoices">
        <template v-if="inv" #badge>
          <StatusBadge v-if="inv.status === 'REFUNDED'" :tone="INVOICE_STATUS.REFUNDED.tone" :label="INVOICE_STATUS.REFUNDED.label" />
          <StatusBadge v-else :tone="PAYMENT_STATUS[inv.paymentStatus].tone" :label="PAYMENT_STATUS[inv.paymentStatus].label" />
        </template>
        <template v-if="inv" #subtitle>
          <span class="num">{{ formatDateTime(inv.date) }}</span> · الكاشير {{ inv.cashierName }}
        </template>
        <template #actions>
          <AppButton v-if="canRefund" :icon="Undo2" :to="`/invoices/${id}/refund`">إرجاع</AppButton>
          <AppButton v-if="canPay" :icon="HandCoins" :to="{ path: '/payments/new', query: { type: 'RECEIVED', party: inv?.customerId, ref: id } }">تسجيل دفعة</AppButton>
          <AppButton variant="primary" :icon="Printer" @click="print">طباعة</AppButton>
        </template>
      </PageHeader>

      <div class="grid items-start gap-5 xl:grid-cols-[1fr_320px]">
        <div class="space-y-5">
          <AppCard padding="none">
            <div v-if="!inv" class="p-4"><SkeletonBlock :lines="6" /></div>
            <table v-else class="w-full text-body">
              <thead class="bg-surface text-xs text-text-secondary">
                <tr class="border-b border-border">
                  <th class="px-4 py-2.5 text-start font-medium">الصنف</th>
                  <th class="px-3 py-2.5 text-start font-medium">الكمية</th>
                  <th class="px-3 py-2.5 text-start font-medium">السعر</th>
                  <th class="px-3 py-2.5 text-start font-medium">المرتجع</th>
                  <th class="px-4 py-2.5 text-start font-medium">الإجمالي</th>
                </tr>
              </thead>
              <tbody class="bg-background">
                <tr v-for="l in inv.lines" :key="l.id" class="border-b border-border last:border-0">
                  <td class="px-4 py-2.5">
                    <RouterLink :to="`/products/${l.productId}`" class="hover:text-primary">{{ l.name }}</RouterLink>
                  </td>
                  <td class="px-3 py-2.5"><span class="num">{{ formatNumber(l.qty) }}</span></td>
                  <td class="px-3 py-2.5"><MoneyText :value="l.price" plain /></td>
                  <td class="px-3 py-2.5">
                    <span v-if="inv.returnedQty[l.id]" class="num text-danger">{{ formatNumber(inv.returnedQty[l.id]) }}</span>
                    <span v-else class="text-text-secondary">—</span>
                  </td>
                  <td class="px-4 py-2.5"><MoneyText :value="round2(l.qty * l.price - l.discount)" /></td>
                </tr>
              </tbody>
            </table>
          </AppCard>

          <AppCard v-if="inv?.refunds.length" title="المرتجعات" padding="none">
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

          <AppCard v-if="inv?.payments.length" title="الدفعات اللاحقة" padding="none">
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
        </div>

        <div class="space-y-4">
          <AppCard title="الملخص" padding="sm">
            <SkeletonBlock v-if="!inv" :lines="5" />
            <dl v-else class="space-y-1.5 text-body">
              <div class="flex justify-between"><dt class="text-text-secondary">المجموع</dt><dd><MoneyText :value="inv.subTotal" /></dd></div>
              <div v-if="inv.discountAmount" class="flex justify-between">
                <dt class="text-text-secondary">الخصم <span class="num">({{ formatNumber(inv.discountRate) }}%)</span></dt>
                <dd>−<MoneyText :value="inv.discountAmount" /></dd>
              </div>
              <div class="flex justify-between">
                <dt class="text-text-secondary">الضريبة <span class="num">({{ formatNumber(inv.taxRate) }}%)</span></dt>
                <dd><MoneyText :value="inv.taxAmount" /></dd>
              </div>
              <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>الإجمالي</dt><dd><MoneyText :value="inv.grandTotal" /></dd></div>
              <div v-if="inv.refundedAmount" class="flex justify-between text-danger"><dt>المرتجع</dt><dd>−<MoneyText :value="inv.refundedAmount" /></dd></div>
              <div class="flex justify-between"><dt class="text-text-secondary">المدفوع</dt><dd><MoneyText :value="inv.paidAmount" /></dd></div>
              <div class="flex justify-between font-medium" :class="inv.outstanding > 0 ? 'text-warning' : ''">
                <dt>المتبقي</dt><dd><MoneyText :value="inv.outstanding" /></dd>
              </div>
            </dl>
          </AppCard>

          <AppCard title="العميل والدفع" padding="sm">
            <SkeletonBlock v-if="!inv" :lines="3" />
            <dl v-else class="space-y-1.5 text-body">
              <div class="flex justify-between gap-3">
                <dt class="text-text-secondary">العميل</dt>
                <dd>
                  <RouterLink v-if="inv.customer" :to="`/customers/${inv.customer.id}`" class="text-primary hover:underline">{{ inv.customer.name }}</RouterLink>
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
                <RouterLink :to="`/accounting/journal/${e.id}`" class="flex items-center gap-2 px-4 py-2 hover:bg-surface-hover">
                  <BookOpen class="size-3.5 shrink-0 text-text-secondary" />
                  <span class="num text-primary">{{ e.number }}</span>
                  <span class="truncate text-xs text-text-secondary">{{ e.description }}</span>
                </RouterLink>
              </li>
            </ul>
          </AppCard>
        </div>
      </div>
    </template>
  </div>
</template>
