<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import QrCode from '../../QrCode.vue';

/** Plan 22 · هندسي — angled charcoal and orange planes top and bottom, numbered line badges, a slanted total. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));

// Decorative planes (clip-path is physical by nature; the shapes are symmetric enough to read in RTL).
const TOP_ORANGE = 'polygon(0 0, 100% 0, 100% 100%, 0 78%)';
const TOP_CHARCOAL = 'polygon(0 0, 100% 0, 100% 86%, 0 100%)';
const BOTTOM_CHARCOAL = 'polygon(0 0, 100% 45%, 100% 100%, 0 100%)';
</script>

<template>
  <article class="relative flex min-h-[296mm] w-[210mm] flex-col overflow-hidden bg-doc-paper text-label leading-relaxed text-doc-ink" dir="rtl">
    <header class="relative h-[62mm]">
      <div class="absolute inset-0 bg-doc-orange" :style="{ clipPath: TOP_ORANGE }" />
      <div class="absolute inset-x-0 top-0 h-[54mm] bg-doc-charcoal" :style="{ clipPath: TOP_CHARCOAL }" />
      <div class="relative flex items-start justify-between px-[14mm] pt-[12mm] text-doc-paper">
        <div class="flex items-center gap-3">
          <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-14 rounded-full bg-doc-paper object-contain p-1" />
          <div v-else class="grid size-14 place-items-center rounded-full bg-doc-orange text-heading font-semibold">{{ d.monogram }}</div>
          <div>
            <p class="text-heading font-semibold">{{ d.store.storeName }}</p>
            <p v-if="d.store.phone" class="num text-tiny opacity-75" dir="ltr">{{ formatDigits(d.store.phone) }}</p>
          </div>
        </div>
        <div class="text-end">
          <p class="text-stat font-semibold leading-none">{{ d.title }}</p>
          <p class="mt-2 text-tiny opacity-75">رقم <span class="num text-label font-medium opacity-100">{{ d.number }}</span></p>
        </div>
      </div>
    </header>

    <div class="flex flex-1 flex-col px-[14mm] pb-[30mm]">
      <!-- Pills -->
      <div class="flex flex-wrap gap-2 text-tiny">
        <span class="rounded-full border border-doc-charcoal px-3 py-1">التاريخ <span class="num font-medium">{{ d.dateTime }}</span></span>
        <span class="rounded-full border border-doc-charcoal px-3 py-1">الدفع <span class="font-medium">{{ d.paymentLabel }}</span></span>
        <span class="rounded-full bg-doc-orange px-3 py-1 text-doc-paper">{{ d.paymentStatus.label }}</span>
      </div>

      <section class="mt-6 grid grid-cols-2 gap-6 text-tiny">
        <div class="border-s-4 border-doc-orange ps-3">
          <p class="text-print-muted">العميل</p>
          <template v-if="d.customer">
            <p class="text-lead font-medium">{{ d.customer.name }}</p>
            <p v-if="d.customerAddress">{{ d.customerAddress }}</p>
            <p v-if="d.customer.vatNumber">{{ d.profile.taxId.label }}: <span class="num">{{ formatDigits(d.customer.vatNumber) }}</span></p>
          </template>
          <p v-else class="text-lead font-medium">عميل نقدي</p>
        </div>
        <div class="border-s-4 border-doc-charcoal ps-3">
          <p class="text-print-muted">البائع</p>
          <p class="text-lead font-medium">{{ d.store.storeName }}</p>
          <p v-if="d.storeAddress">{{ d.storeAddress }}</p>
          <p v-if="d.store.vatNumber">{{ d.profile.taxId.label }}: <span class="num">{{ formatDigits(d.store.vatNumber) }}</span></p>
        </div>
      </section>

      <ul class="mt-7 space-y-2">
        <li v-for="l in d.lines" :key="l.id" class="grid grid-cols-[auto_1fr_auto_auto] items-center gap-4 rounded-lg bg-print-fill px-3 py-2">
          <span class="num grid size-7 place-items-center rounded-full bg-doc-orange text-tiny font-semibold text-doc-paper">{{ formatNumber(l.no) }}</span>
          <span class="text-body font-medium">{{ l.name }}</span>
          <span class="num text-tiny text-print-muted">{{ formatNumber(l.qty) }} × {{ formatNumber(l.price, 2) }}</span>
          <span class="w-28 text-end text-body font-semibold"><MoneyText :value="l.total" plain /></span>
        </li>
      </ul>

      <div class="mt-6 flex items-start justify-between gap-8">
        <div class="max-w-[80mm] space-y-2 text-tiny text-print-muted">
          <p>{{ d.amountInWords }}</p>
          <p v-if="d.note">ملاحظات: {{ d.note }}</p>
          <QrCode v-if="d.qr" :value="d.qr" size="24mm" :border="1" />
        </div>
        <div class="w-[80mm]">
          <dl class="space-y-1 text-body">
            <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between">
              <dt class="text-print-muted">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
              <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
            </div>
          </dl>
          <div class="mt-3 -skew-x-12 bg-doc-orange px-4 py-2.5 text-doc-paper">
            <div class="flex skew-x-12 items-center justify-between">
              <span class="text-body font-medium">الإجمالي</span>
              <span class="text-heading font-semibold"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <footer class="absolute inset-x-0 bottom-0 h-[24mm] bg-doc-charcoal text-tiny text-doc-paper" :style="{ clipPath: BOTTOM_CHARCOAL }">
      <div class="flex h-full items-end justify-between px-[14mm] pb-4">
        <span>{{ d.footer || `الكاشير: ${d.cashier}` }}</span>
        <span v-if="d.sample" class="font-medium text-doc-orange">نموذج اختبار طباعة، ليست فاتورة حقيقية</span>
      </div>
    </footer>
  </article>
</template>
