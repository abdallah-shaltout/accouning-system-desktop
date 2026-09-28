<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import QrCode from '../../QrCode.vue';

/** Plan 22 · شريط علوي — a dark slate band opens the page with the grand total in amber; everything below stays airy. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="flex min-h-[296mm] w-[210mm] flex-col bg-doc-paper text-label leading-relaxed text-doc-ink" dir="rtl">
    <header class="bg-doc-slate px-[14mm] pb-8 pt-[12mm] text-doc-paper">
      <div class="flex items-center gap-3">
        <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-12 rounded-md bg-doc-paper object-contain p-1" />
        <div>
          <p class="text-heading font-semibold">{{ d.store.storeName }}</p>
          <p class="text-tiny opacity-70">{{ d.title }}<template v-if="d.titleEn"> · <span dir="ltr">{{ d.titleEn }}</span></template></p>
        </div>
      </div>
      <div class="mt-8 flex items-end justify-between gap-6">
        <div>
          <p class="text-tiny opacity-70">المبلغ الإجمالي</p>
          <p class="text-display font-semibold leading-none text-doc-amber"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></p>
        </div>
        <p class="max-w-[80mm] text-end text-tiny opacity-70">{{ d.amountInWords }}</p>
      </div>
    </header>

    <!-- Meta strip -->
    <dl class="grid grid-cols-4 border-b border-print-rule bg-doc-amber/15 px-[14mm] py-3 text-tiny">
      <div><dt class="text-print-muted">رقم الفاتورة</dt><dd class="num text-label font-medium">{{ d.number }}</dd></div>
      <div><dt class="text-print-muted">التاريخ</dt><dd class="num text-label">{{ d.dateTime }}</dd></div>
      <div><dt class="text-print-muted">طريقة الدفع</dt><dd class="text-label">{{ d.paymentLabel }}</dd></div>
      <div><dt class="text-print-muted">الحالة</dt><dd class="text-label font-medium">{{ d.paymentStatus.label }}</dd></div>
    </dl>

    <div class="flex flex-1 flex-col px-[14mm] py-8">
      <section class="grid grid-cols-2 gap-10 text-tiny">
        <div>
          <p class="mb-1 text-print-muted">من</p>
          <p class="text-body font-medium">{{ d.store.storeName }}</p>
          <p v-if="d.storeAddress" class="text-print-muted">{{ d.storeAddress }}</p>
          <p v-if="d.store.phone" class="num text-print-muted" dir="ltr">{{ formatDigits(d.store.phone) }}</p>
          <p v-if="d.store.vatNumber">{{ d.profile.taxId.label }}: <span class="num">{{ formatDigits(d.store.vatNumber) }}</span></p>
        </div>
        <div>
          <p class="mb-1 text-print-muted">إلى</p>
          <template v-if="d.customer">
            <p class="text-body font-medium">{{ d.customer.name }}</p>
            <p v-if="d.customerAddress" class="text-print-muted">{{ d.customerAddress }}</p>
            <p v-if="d.customer.phone" class="num text-print-muted" dir="ltr">{{ formatDigits(d.customer.phone) }}</p>
            <p v-if="d.customer.vatNumber">{{ d.profile.taxId.label }}: <span class="num">{{ formatDigits(d.customer.vatNumber) }}</span></p>
          </template>
          <p v-else class="text-body font-medium">عميل نقدي</p>
        </div>
      </section>

      <table class="mt-8 w-full text-body">
        <thead>
          <tr class="text-tiny text-print-muted">
            <th class="pb-2 text-start font-medium">الصنف</th>
            <th class="pb-2 text-center font-medium">الكمية</th>
            <th class="pb-2 text-center font-medium">السعر</th>
            <th class="pb-2 text-center font-medium">الضريبة</th>
            <th class="pb-2 text-end font-medium">الإجمالي</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="l in d.lines" :key="l.id" class="border-t border-print-rule">
            <td class="py-3 font-medium">{{ l.name }}</td>
            <td class="py-3 text-center"><span class="num">{{ formatNumber(l.qty) }}</span></td>
            <td class="py-3 text-center"><MoneyText :value="l.price" plain /></td>
            <td class="py-3 text-center text-print-muted"><MoneyText :value="l.vat" plain /></td>
            <td class="py-3 text-end font-medium"><MoneyText :value="l.total" plain /></td>
          </tr>
        </tbody>
      </table>

      <div class="mt-6 flex items-start justify-between gap-8 border-t-2 border-doc-slate pt-4">
        <div class="max-w-[90mm] text-tiny text-print-muted">
          <p v-if="d.note">ملاحظات: {{ d.note }}</p>
          <p>الكاشير: {{ d.cashier }}</p>
        </div>
        <dl class="w-[76mm] space-y-1.5 text-body">
          <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between">
            <dt class="text-print-muted">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
            <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
          </div>
          <div class="flex justify-between rounded-md bg-doc-slate px-3 py-2 text-lead font-semibold text-doc-paper">
            <dt>الإجمالي</dt>
            <dd class="text-doc-amber"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></dd>
          </div>
        </dl>
      </div>
    </div>

    <footer class="flex items-center justify-between gap-6 bg-doc-slate px-[14mm] py-4 text-tiny text-doc-paper">
      <div>
        <p v-if="d.footer" class="text-label">{{ d.footer }}</p>
        <p v-if="d.sample" class="font-medium text-doc-amber">— نموذج اختبار طباعة، ليست فاتورة حقيقية —</p>
      </div>
      <div v-if="d.qr" class="rounded bg-doc-paper p-1"><QrCode :value="d.qr" size="22mm" :border="1" /></div>
    </footer>
  </article>
</template>
