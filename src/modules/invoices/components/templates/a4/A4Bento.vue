<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import QrCode from '../../QrCode.vue';

/** Plan 22 · مربعات — every group of facts gets its own tile (brand, customer, payment, dates); the total is the one dark tile. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="flex min-h-[296mm] w-[210mm] flex-col gap-3 bg-doc-paper p-[11mm] text-label leading-relaxed text-doc-ink" dir="rtl">
    <!-- Row 1 -->
    <div class="grid grid-cols-[1.6fr_1fr] gap-3">
      <div class="flex items-center gap-4 rounded-2xl bg-doc-sage px-6 py-5 text-doc-paper">
        <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-14 rounded-xl bg-doc-paper object-contain p-1" />
        <div v-else class="grid size-14 place-items-center rounded-xl bg-doc-paper/15 text-heading font-semibold">{{ d.monogram }}</div>
        <div>
          <p class="text-heading font-semibold">{{ d.store.storeName }}</p>
          <p v-if="d.storeAddress" class="text-tiny opacity-80">{{ d.storeAddress }}</p>
          <p class="text-tiny opacity-80">
            <span v-if="d.store.phone" class="num" dir="ltr">{{ formatDigits(d.store.phone) }}</span>
          </p>
        </div>
      </div>
      <div class="flex flex-col justify-between rounded-2xl border border-doc-sage/30 px-6 py-5">
        <p class="text-lead font-semibold text-doc-sage">{{ d.title }}</p>
        <div>
          <p class="text-tiny text-print-muted">رقم الفاتورة</p>
          <p class="num text-heading-sm font-semibold">{{ d.number }}</p>
        </div>
      </div>
    </div>

    <!-- Row 2 -->
    <div class="grid grid-cols-3 gap-3 text-tiny">
      <div class="rounded-2xl bg-doc-sage-soft px-5 py-4">
        <p class="text-print-muted">العميل</p>
        <template v-if="d.customer">
          <p class="text-body font-medium">{{ d.customer.name }}</p>
          <p v-if="d.customer.phone" class="num text-print-muted" dir="ltr">{{ formatDigits(d.customer.phone) }}</p>
          <p v-if="d.customer.vatNumber" class="num">{{ formatDigits(d.customer.vatNumber) }}</p>
        </template>
        <p v-else class="text-body font-medium">عميل نقدي</p>
      </div>
      <div class="rounded-2xl bg-doc-sage-soft px-5 py-4">
        <p class="text-print-muted">الدفع</p>
        <p class="text-body font-medium">{{ d.paymentLabel }}</p>
        <p>{{ d.paymentStatus.label }}</p>
        <p v-if="d.outstanding > 0">المتبقي: <MoneyText :value="d.outstanding" :currency="d.currency" /></p>
      </div>
      <div class="rounded-2xl bg-doc-sage-soft px-5 py-4">
        <p class="text-print-muted">التاريخ</p>
        <p class="num text-body font-medium">{{ d.date }}</p>
        <p class="num">{{ d.time }}</p>
        <p v-if="d.dueDate">الاستحقاق: <span class="num">{{ d.dueDate }}</span></p>
      </div>
    </div>

    <!-- Lines -->
    <div class="overflow-hidden rounded-2xl border border-doc-sage/30">
      <table class="w-full text-body">
        <thead class="bg-doc-sage-soft text-tiny text-doc-sage">
          <tr>
            <th class="px-5 py-2.5 text-start font-medium">الصنف</th>
            <th class="px-3 py-2.5 text-center font-medium">الكمية</th>
            <th class="px-3 py-2.5 text-center font-medium">السعر</th>
            <th class="px-3 py-2.5 text-center font-medium">الضريبة</th>
            <th class="px-5 py-2.5 text-end font-medium">الإجمالي</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="l in d.lines" :key="l.id" class="border-t border-doc-sage/15">
            <td class="px-5 py-3">{{ l.name }}</td>
            <td class="px-3 py-3 text-center">
              <span class="num inline-block min-w-8 rounded-full bg-doc-sage-soft px-2 text-tiny">{{ formatNumber(l.qty) }}</span>
            </td>
            <td class="px-3 py-3 text-center"><MoneyText :value="l.price" plain /></td>
            <td class="px-3 py-3 text-center text-print-muted"><MoneyText :value="l.vat" plain /></td>
            <td class="px-5 py-3 text-end font-medium"><MoneyText :value="l.total" plain /></td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Row 4 -->
    <div class="grid grid-cols-[1fr_1fr_auto] gap-3">
      <div class="space-y-1.5 rounded-2xl border border-doc-sage/30 px-5 py-4 text-body">
        <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between">
          <span class="text-print-muted">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></span>
          <span><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></span>
        </div>
      </div>
      <div class="flex flex-col justify-between rounded-2xl bg-doc-sage px-5 py-4 text-doc-paper">
        <p class="text-tiny opacity-80">الإجمالي</p>
        <p class="text-stat font-semibold leading-tight"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></p>
        <p class="text-caption opacity-80">{{ d.amountInWords }}</p>
      </div>
      <div v-if="d.qr" class="grid place-items-center rounded-2xl border border-doc-sage/30 p-3"><QrCode :value="d.qr" size="30mm" :border="0" /></div>
    </div>

    <div v-if="d.note" class="rounded-2xl bg-doc-sage-soft px-5 py-3 text-tiny">ملاحظات: {{ d.note }}</div>

    <footer class="mt-auto flex items-center justify-between rounded-2xl border border-doc-sage/30 px-5 py-3 text-tiny text-print-muted">
      <span>{{ d.footer || `الكاشير: ${d.cashier}` }}</span>
      <span v-if="d.store.vatNumber">{{ d.profile.taxId.label }}: <span class="num text-doc-ink">{{ formatDigits(d.store.vatNumber) }}</span></span>
      <span v-if="d.sample" class="font-medium text-doc-ink">نموذج اختبار طباعة</span>
    </footer>
  </article>
</template>
