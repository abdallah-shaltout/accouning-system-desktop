<script setup lang="ts">
import { computed } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDateTime, formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { SALE_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { changeDue, round2 } from '../helpers/totals';
import { zatcaQrBase64 } from '../helpers/zatcaQr';
import type { PrintData } from '../services/invoiceService';
import QrCode from './QrCode.vue';

/** Narrow receipt for 58mm / 80mm thermal printers: one column, condensed lines, centered QR. */
const props = defineProps<{ data: PrintData; width: 58 | 80 }>();

const inv = computed(() => props.data.invoice);
const s = computed(() => props.data.settings);
const narrow = computed(() => props.width === 58);
const qr = computed(() =>
  zatcaQrBase64({
    sellerName: s.value.storeName,
    vatNumber: s.value.vatNumber ?? '',
    timestamp: inv.value.date,
    invoiceTotal: inv.value.grandTotal,
    vatTotal: inv.value.taxAmount,
  }),
);
const change = computed(() => (inv.value.tenderedAmount ? changeDue(inv.value.grandTotal, inv.value.tenderedAmount) : 0));
</script>

<template>
  <article
    class="mx-auto bg-white text-black"
    :class="narrow ? 'text-[10.5px] leading-snug' : 'text-[12px] leading-normal'"
    :style="{ width: `${width}mm`, padding: narrow ? '2mm 2.5mm' : '3mm 4mm' }"
    dir="rtl"
  >
    <header class="text-center">
      <img v-if="s.logo" :src="s.logo" alt="" class="mx-auto mb-1 h-12 object-contain grayscale" />
      <p class="font-semibold" :class="narrow ? 'text-[13px]' : 'text-[15px]'">{{ s.storeName }}</p>
      <p v-if="s.address">{{ s.address }}</p>
      <p v-if="s.phone" class="num text-center">{{ formatDigits(s.phone) }}</p>
      <p v-if="s.vatNumber">الرقم الضريبي: <span class="num">{{ formatDigits(s.vatNumber) }}</span></p>
    </header>

    <div class="my-1.5 border-t border-dashed border-black" />
    <p class="text-center font-semibold">{{ data.customer?.vatNumber ? 'فاتورة ضريبية' : 'فاتورة ضريبية مبسطة' }}</p>
    <dl class="mt-1 space-y-0.5">
      <div class="flex justify-between gap-2"><dt>رقم الفاتورة</dt><dd class="num">{{ formatDigits(inv.number) }}</dd></div>
      <div class="flex justify-between gap-2"><dt>التاريخ</dt><dd class="num">{{ formatDateTime(inv.date) }}</dd></div>
      <div class="flex justify-between gap-2"><dt>الكاشير</dt><dd>{{ data.cashierName }}</dd></div>
      <div v-if="data.customer" class="flex justify-between gap-2"><dt>العميل</dt><dd class="truncate">{{ data.customer.name }}</dd></div>
    </dl>

    <div class="my-1.5 border-t border-dashed border-black" />
    <ul class="space-y-1">
      <li v-for="l in inv.lines" :key="l.id">
        <p>{{ l.name }}</p>
        <div class="flex justify-between gap-2">
          <span class="num text-print-muted">{{ formatNumber(l.qty) }} × {{ formatNumber(l.price, 2) }}</span>
          <MoneyText :value="round2(l.qty * l.price - l.discount)" plain />
        </div>
      </li>
    </ul>

    <div class="my-1.5 border-t border-dashed border-black" />
    <dl class="space-y-0.5">
      <div class="flex justify-between"><dt>المجموع</dt><dd><MoneyText :value="inv.subTotal" plain /></dd></div>
      <div v-if="inv.discountAmount > 0" class="flex justify-between">
        <dt>الخصم <span class="num">{{ formatNumber(inv.discountRate) }}%</span></dt><dd>−<MoneyText :value="inv.discountAmount" plain /></dd>
      </div>
      <div class="flex justify-between">
        <dt>ضريبة القيمة المضافة <span class="num">{{ formatNumber(inv.taxRate) }}%</span></dt><dd><MoneyText :value="inv.taxAmount" plain /></dd>
      </div>
      <div class="flex justify-between border-t border-black pt-1 font-semibold" :class="narrow ? 'text-[13px]' : 'text-[15px]'">
        <dt>الإجمالي</dt><dd><MoneyText :value="inv.grandTotal" /></dd>
      </div>
      <div class="flex justify-between pt-0.5"><dt>الدفع</dt><dd>{{ SALE_METHOD_LABEL[inv.paymentMethod] }}</dd></div>
      <template v-if="inv.tenderedAmount">
        <div class="flex justify-between"><dt>المستلم</dt><dd><MoneyText :value="inv.tenderedAmount" plain /></dd></div>
        <div class="flex justify-between"><dt>الباقي</dt><dd><MoneyText :value="change" plain /></dd></div>
      </template>
      <div v-if="inv.paidAmount < inv.grandTotal" class="flex justify-between font-medium">
        <dt>المتبقي على الحساب</dt><dd><MoneyText :value="round2(inv.grandTotal - inv.paidAmount)" plain /></dd>
      </div>
    </dl>

    <div class="my-2 flex justify-center">
      <QrCode :value="qr" :size="narrow ? '28mm' : '34mm'" :border="1" />
    </div>
    <p v-if="s.receiptFooter" class="text-center">{{ s.receiptFooter }}</p>
    <p v-if="data.sample" class="mt-1 text-center font-medium">— اختبار طباعة —</p>
  </article>
</template>
