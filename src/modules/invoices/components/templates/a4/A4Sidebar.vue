<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import QrCode from '../../QrCode.vue';

/** Plan 22 · عمود جانبي — a teal start-side column carries the brand, the meta and the amount due; the lines get the whole main column. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="grid min-h-[296mm] w-[210mm] grid-cols-[64mm_1fr] bg-doc-paper text-label leading-relaxed text-doc-ink" dir="rtl">
    <!-- Column -->
    <aside class="flex flex-col gap-7 bg-doc-teal px-[9mm] py-[14mm] text-doc-paper">
      <div>
        <img v-if="d.store.logo" :src="d.store.logo" alt="" class="mb-3 size-16 rounded-lg bg-doc-paper object-contain p-1.5" />
        <p class="text-heading font-semibold leading-snug">{{ d.store.storeName }}</p>
        <p v-if="d.storeAddress" class="mt-1 text-tiny opacity-80">{{ d.storeAddress }}</p>
        <p v-if="d.store.phone" class="num text-tiny opacity-80" dir="ltr">{{ formatDigits(d.store.phone) }}</p>
        <p v-if="d.store.vatNumber" class="mt-2 text-tiny opacity-80">{{ d.profile.taxId.label }}<br /><span class="num text-label opacity-100">{{ formatDigits(d.store.vatNumber) }}</span></p>
      </div>

      <div>
        <p class="text-tiny opacity-70">فاتورة إلى</p>
        <template v-if="d.customer">
          <p class="text-lead font-medium">{{ d.customer.name }}</p>
          <p v-if="d.customerAddress" class="text-tiny opacity-80">{{ d.customerAddress }}</p>
          <p v-if="d.customer.vatNumber" class="num text-tiny opacity-80">{{ formatDigits(d.customer.vatNumber) }}</p>
          <p v-if="d.customer.phone" class="num text-tiny opacity-80" dir="ltr">{{ formatDigits(d.customer.phone) }}</p>
        </template>
        <p v-else class="text-lead font-medium">عميل نقدي</p>
      </div>

      <dl class="space-y-3 text-tiny">
        <div><dt class="opacity-70">رقم الفاتورة</dt><dd class="num text-label font-medium">{{ d.number }}</dd></div>
        <div><dt class="opacity-70">التاريخ</dt><dd class="num text-label">{{ d.dateTime }}</dd></div>
        <div v-if="d.dueDate"><dt class="opacity-70">تاريخ الاستحقاق</dt><dd class="num text-label">{{ d.dueDate }}</dd></div>
        <div><dt class="opacity-70">طريقة الدفع</dt><dd class="text-label">{{ d.paymentLabel }}</dd></div>
      </dl>

      <div class="mt-auto">
        <p class="text-tiny opacity-70">{{ d.outstanding > 0 ? 'المبلغ المستحق' : 'الإجمالي المدفوع' }}</p>
        <p class="text-stat font-semibold leading-tight"><MoneyText :value="d.outstanding > 0 ? d.outstanding : d.inv.grandTotal" :currency="d.currency" /></p>
        <div v-if="d.qr" class="mt-4 inline-block rounded-lg bg-doc-paper p-1.5"><QrCode :value="d.qr" size="28mm" :border="1" /></div>
      </div>
    </aside>

    <!-- Main -->
    <main class="flex flex-col px-[12mm] py-[14mm]">
      <header class="flex items-baseline justify-between gap-4">
        <h1 class="text-display font-semibold leading-none text-doc-teal">فاتورة</h1>
        <div class="text-end">
          <p class="text-body font-medium">{{ d.title }}</p>
          <p v-if="d.titleEn" class="text-caption tracking-wide text-print-muted" dir="ltr">{{ d.titleEn }}</p>
        </div>
      </header>

      <div class="mt-8 grid grid-cols-[1fr_auto_auto] gap-x-6 border-b-2 border-doc-teal pb-2 text-tiny text-print-muted">
        <span>الصنف</span>
        <span class="w-20 text-center">الكمية</span>
        <span class="w-24 text-end">المبلغ</span>
      </div>
      <ul>
        <li v-for="l in d.lines" :key="l.id" class="grid grid-cols-[1fr_auto_auto] items-center gap-x-6 border-b border-print-rule py-3">
          <div>
            <p class="text-body font-medium">{{ l.name }}</p>
            <p class="text-tiny text-print-muted">سعر الوحدة <MoneyText :value="l.price" plain /></p>
          </div>
          <span class="num w-20 text-center text-body">{{ formatNumber(l.qty) }}</span>
          <span class="w-24 text-end text-body"><MoneyText :value="l.amount" plain /></span>
        </li>
      </ul>

      <dl class="ms-auto mt-6 w-[80mm] space-y-1.5 text-body">
        <div v-for="r in d.totals" :key="r.key" class="flex justify-between gap-4" :class="r.key === 'grand' ? 'mt-2 border-t-2 border-doc-teal pt-2 text-lead font-semibold text-doc-teal' : ''">
          <dt :class="r.key === 'grand' ? '' : 'text-print-muted'">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
          <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
        </div>
      </dl>
      <p class="ms-auto mt-2 max-w-[80mm] text-end text-tiny text-print-muted">{{ d.amountInWords }}</p>

      <p v-if="d.note" class="mt-6 rounded-lg bg-doc-teal-soft px-4 py-3 text-tiny">{{ d.note }}</p>

      <footer class="mt-auto pt-8 text-tiny text-print-muted">
        <p v-if="d.footer" class="text-body text-doc-teal">{{ d.footer }}</p>
        <p>الكاشير: {{ d.cashier }}</p>
        <p v-if="d.sample" class="mt-1 font-medium text-doc-ink">— نموذج اختبار طباعة، ليست فاتورة حقيقية —</p>
      </footer>
    </main>
  </article>
</template>
