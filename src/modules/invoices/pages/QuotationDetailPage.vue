<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §2 "Convert → invoice copies everything"). */
import { computed, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { FileCheck, Send, X } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import TenderDialog from '../components/TenderDialog.vue';
import { convertQuotationToInvoice, getQuotation, setQuotationStatus } from '../services/invoiceService';
import type { Tender } from '../types';

const route = useRoute();
const router = useRouter();
const toast = useToast();
const id = String(route.params.id);
const { data, error, reload } = useAsync(() => getQuotation(id));
const q = computed(() => data.value);
const tenderOpen = ref(false);
const converting = ref(false);

const STATUS_LABEL: Record<string, { label: string; tone: 'neutral' | 'primary' | 'success' | 'danger' | 'warning' }> = {
  DRAFT: { label: 'مسودة', tone: 'neutral' },
  SENT: { label: 'مُرسل', tone: 'primary' },
  ACCEPTED: { label: 'مقبول', tone: 'success' },
  REJECTED: { label: 'مرفوض', tone: 'danger' },
  EXPIRED: { label: 'منتهي', tone: 'warning' },
};

async function setStatus(status: 'SENT' | 'REJECTED') {
  await setQuotationStatus(id, status);
  reload();
}

async function convert(payment: { tenders: Tender[]; paidAmount: number; tenderedAmount?: number }) {
  converting.value = true;
  try {
    const invoice = await convertQuotationToInvoice(id, { paymentMethod: 'cash', paidAmount: payment.paidAmount, tenderedAmount: payment.tenderedAmount });
    toast.success('تم تحويل العرض إلى فاتورة', invoice.number);
    router.push(`/invoices/${invoice.id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    converting.value = false;
    tenderOpen.value = false;
  }
}

const draft = computed(() => ({
  customerId: q.value?.customerId,
  discountRate: q.value?.discountRate ?? 0,
  lines: (q.value?.lines ?? []).map((l) => ({ productId: l.productId, qty: l.qty, price: l.price, discount: l.discount, taxId: l.taxId })),
}));
</script>

<template>
  <div>
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <template v-else>
      <PageHeader :title="q ? `عرض سعر ${q.number}` : '…'" back="/sales/quotations">
        <template v-if="q" #badge><StatusBadge :tone="STATUS_LABEL[q.status].tone" :label="STATUS_LABEL[q.status].label" /></template>
        <template #actions>
          <template v-if="q && q.status === 'DRAFT'">
            <AppButton :icon="Send" @click="setStatus('SENT')">تحديد كمُرسل</AppButton>
          </template>
          <template v-if="q && !q.convertedInvoiceId && q.status !== 'REJECTED'">
            <AppButton :icon="X" @click="setStatus('REJECTED')">رفض</AppButton>
            <AppButton variant="primary" :icon="FileCheck" @click="tenderOpen = true">تحويل إلى فاتورة</AppButton>
          </template>
          <AppButton v-if="q?.convertedInvoiceId" variant="primary" :to="`/invoices/${q.convertedInvoiceId}`">عرض الفاتورة</AppButton>
        </template>
      </PageHeader>

      <AppCard padding="none">
        <div v-if="!q" class="p-4"><SkeletonBlock :lines="5" /></div>
        <table v-else class="w-full text-body">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr><th class="px-4 py-2 text-start">الصنف</th><th class="px-3 py-2 text-start">الكمية</th><th class="px-3 py-2 text-start">السعر</th><th class="px-4 py-2 text-start">الإجمالي</th></tr>
          </thead>
          <tbody>
            <tr v-for="l in q.lines" :key="l.id" class="border-t border-border">
              <td class="px-4 py-2">{{ l.name }}</td>
              <td class="px-3 py-2"><span class="num">{{ formatNumber(l.qty) }}</span></td>
              <td class="px-3 py-2"><MoneyText :value="l.price" plain /></td>
              <td class="px-4 py-2"><MoneyText :value="(l.net ?? 0) + (l.vat ?? 0)" /></td>
            </tr>
          </tbody>
        </table>
      </AppCard>

      <AppCard v-if="q" title="الإجمالي" padding="sm" class="mt-4 max-w-sm">
        <dl class="space-y-1.5 text-body">
          <div class="flex justify-between"><dt class="text-text-secondary">التاريخ</dt><dd class="num">{{ formatDateTime(q.date) }}</dd></div>
          <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>الإجمالي</dt><dd><MoneyText :value="q.grandTotal" /></dd></div>
        </dl>
      </AppCard>
    </template>

    <TenderDialog v-model:open="tenderOpen" :total="q?.grandTotal ?? 0" :customer-name="q?.customerName" :has-customer="!!q?.customerId" :draft="draft" :submitting="converting" @confirm="convert" />
  </div>
</template>
