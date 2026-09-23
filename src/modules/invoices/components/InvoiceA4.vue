<script setup lang="ts">
import { computed } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDateTime, formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { SALE_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { round2 } from '../helpers/totals';
import { zatcaQrBase64 } from '../helpers/zatcaQr';
import type { PrintData } from '../services/invoiceService';
import QrCode from './QrCode.vue';

/** A4 tax invoice. Black-on-white by design (prints the same from light or dark mode). */
const props = defineProps<{ data: PrintData }>();

const inv = computed(() => props.data.invoice);
const s = computed(() => props.data.settings);
const isB2B = computed(() => !!props.data.customer?.vatNumber);
const qr = computed(() =>
  zatcaQrBase64({
    sellerName: s.value.storeName,
    vatNumber: s.value.vatNumber ?? '',
    timestamp: inv.value.date,
    invoiceTotal: inv.value.grandTotal,
    vatTotal: inv.value.taxAmount,
  }),
);
const lineNet = (l: { qty: number; price: number; discount: number }) => round2(l.qty * l.price - l.discount);
const outstanding = computed(() => Math.max(0, round2(inv.value.grandTotal - inv.value.refundedAmount - inv.value.paidAmount)));
</script>

<template>
  <article class="mx-auto flex min-h-[273mm] w-[186mm] flex-col bg-white p-0 text-[12px] leading-relaxed text-black" dir="rtl">
    <!-- Header -->
    <header class="flex items-start justify-between gap-6 border-b-2 border-black pb-4">
      <div class="flex items-start gap-3">
        <img v-if="s.logo" :src="s.logo" alt="" class="size-16 object-contain" />
        <div>
          <h1 class="text-[18px] font-semibold">{{ s.storeName }}</h1>
          <p v-if="s.address" class="text-[11px] text-print-muted">{{ s.address }}</p>
          <p class="text-[11px] text-print-muted">
            <template v-if="s.phone">هاتف: <span class="num">{{ formatDigits(s.phone) }}</span></template>
            <template v-if="s.commercialRegister"> · س.ت: <span class="num">{{ formatDigits(s.commercialRegister) }}</span></template>
          </p>
          <p v-if="s.vatNumber" class="text-[11px]">الرقم الضريبي: <span class="num font-medium">{{ formatDigits(s.vatNumber) }}</span></p>
        </div>
      </div>
      <div class="text-start">
        <p class="text-[17px] font-semibold">{{ isB2B ? 'فاتورة ضريبية' : 'فاتورة ضريبية مبسطة' }}</p>
        <p class="text-[10px] tracking-wide text-print-muted" dir="ltr">{{ isB2B ? 'TAX INVOICE' : 'SIMPLIFIED TAX INVOICE' }}</p>
        <table class="mt-2 text-[11px]">
          <tbody>
            <tr><td class="pe-3 text-print-muted">رقم الفاتورة</td><td class="num font-medium">{{ formatDigits(inv.number) }}</td></tr>
            <tr><td class="pe-3 text-print-muted">التاريخ</td><td class="num">{{ formatDateTime(inv.date) }}</td></tr>
          </tbody>
        </table>
      </div>
    </header>

    <!-- Parties -->
    <section class="grid grid-cols-2 gap-6 border-b border-print-rule py-3 text-[11px]">
      <div>
        <p class="mb-1 font-medium text-print-muted">العميل</p>
        <template v-if="data.customer">
          <p class="text-[12px] font-medium">{{ data.customer.name }}</p>
          <p v-if="data.customer.vatNumber">الرقم الضريبي: <span class="num">{{ formatDigits(data.customer.vatNumber) }}</span></p>
          <p v-if="data.customer.address" class="text-print-muted">{{ data.customer.address }}</p>
          <p v-if="data.customer.phone" class="num text-print-muted">{{ formatDigits(data.customer.phone) }}</p>
        </template>
        <p v-else>عميل نقدي</p>
      </div>
      <div>
        <p class="mb-1 font-medium text-print-muted">الدفع</p>
        <p>طريقة الدفع: {{ SALE_METHOD_LABEL[inv.paymentMethod] }}</p>
        <p>الكاشير: {{ data.cashierName }}</p>
        <p v-if="outstanding > 0" class="font-medium">المتبقي على العميل: <MoneyText :value="outstanding" /></p>
      </div>
    </section>

    <!-- Lines -->
    <table class="mt-4 w-full border-collapse text-[11px]">
      <thead>
        <tr class="border-y border-black bg-print-fill">
          <th class="w-8 px-2 py-1.5 text-start font-medium">#</th>
          <th class="px-2 py-1.5 text-start font-medium">الصنف</th>
          <th class="px-2 py-1.5 text-start font-medium">الكمية</th>
          <th class="px-2 py-1.5 text-start font-medium">سعر الوحدة</th>
          <th class="px-2 py-1.5 text-start font-medium">المبلغ قبل الضريبة</th>
          <th class="px-2 py-1.5 text-start font-medium">الضريبة <span class="num">{{ formatNumber(inv.taxRate) }}%</span></th>
          <th class="px-2 py-1.5 text-start font-medium">الإجمالي</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(l, i) in inv.lines" :key="l.id" class="border-b border-print-rule">
          <td class="px-2 py-1.5"><span class="num">{{ formatNumber(i + 1) }}</span></td>
          <td class="px-2 py-1.5">{{ l.name }}</td>
          <td class="px-2 py-1.5"><span class="num">{{ formatNumber(l.qty) }}</span></td>
          <td class="px-2 py-1.5"><MoneyText :value="l.price" plain /></td>
          <td class="px-2 py-1.5"><MoneyText :value="lineNet(l)" plain /></td>
          <td class="px-2 py-1.5"><MoneyText :value="round2((lineNet(l) * (1 - inv.discountRate / 100) * inv.taxRate) / 100)" plain /></td>
          <td class="px-2 py-1.5"><MoneyText :value="round2(lineNet(l) * (1 - inv.discountRate / 100) * (1 + inv.taxRate / 100))" plain /></td>
        </tr>
      </tbody>
    </table>

    <!-- Totals -->
    <section class="mt-4 flex justify-end">
      <table class="w-[80mm] text-[11px]">
        <tbody>
          <tr><td class="py-1 text-print-muted">الإجمالي قبل الخصم</td><td class="py-1 text-end"><MoneyText :value="inv.subTotal" /></td></tr>
          <tr v-if="inv.discountAmount > 0">
            <td class="py-1 text-print-muted">الخصم (<span class="num">{{ formatNumber(inv.discountRate) }}%</span>)</td>
            <td class="py-1 text-end">− <MoneyText :value="inv.discountAmount" /></td>
          </tr>
          <tr><td class="py-1 text-print-muted">الإجمالي الخاضع للضريبة</td><td class="py-1 text-end"><MoneyText :value="round2(inv.subTotal - inv.discountAmount)" /></td></tr>
          <tr>
            <td class="py-1 text-print-muted">ضريبة القيمة المضافة (<span class="num">{{ formatNumber(inv.taxRate) }}%</span>)</td>
            <td class="py-1 text-end"><MoneyText :value="inv.taxAmount" /></td>
          </tr>
          <tr class="border-t-2 border-black text-[13px] font-semibold">
            <td class="py-1.5">الإجمالي شامل الضريبة</td>
            <td class="py-1.5 text-end"><MoneyText :value="inv.grandTotal" /></td>
          </tr>
          <tr v-if="inv.refundedAmount > 0">
            <td class="py-1 text-print-muted">المرتجع</td>
            <td class="py-1 text-end">− <MoneyText :value="inv.refundedAmount" /></td>
          </tr>
        </tbody>
      </table>
    </section>

    <p v-if="inv.note" class="mt-4 text-[11px] text-print-muted">ملاحظات: {{ inv.note }}</p>

    <!-- Footer: QR at the bottom-left (the end edge in RTL) -->
    <footer class="mt-auto flex items-end justify-between gap-6 border-t border-print-rule pt-4">
      <div class="text-[11px] text-print-muted">
        <p v-if="s.receiptFooter">{{ s.receiptFooter }}</p>
        <p v-if="data.sample" class="mt-1 font-medium text-black">— نموذج اختبار طباعة، ليست فاتورة حقيقية —</p>
      </div>
      <QrCode :value="qr" size="30mm" />
    </footer>
  </article>
</template>
