<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import QrCode from '../../QrCode.vue';

/** Plan 22 · مضغوط — built for long invoices: a one-line header, dense zebra rows with every column, and a totals strip instead of a totals block. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="flex min-h-[296mm] w-[210mm] flex-col bg-doc-paper px-[10mm] py-[9mm] text-tiny leading-normal text-doc-ink" dir="rtl">
    <header class="flex items-center gap-4 border-b-2 border-doc-ink pb-2">
      <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-9 object-contain" />
      <p class="text-lead font-semibold">{{ d.store.storeName }}</p>
      <span class="h-5 w-px bg-print-rule" />
      <p class="text-body font-medium">{{ d.title }}</p>
      <p class="ms-auto">رقم <span class="num text-body font-semibold">{{ d.number }}</span></p>
      <p class="num">{{ d.dateTime }}</p>
    </header>

    <div class="flex flex-wrap gap-x-5 gap-y-0.5 border-b border-print-rule py-1.5 text-print-muted">
      <span>العميل: <span class="text-doc-ink">{{ d.customer?.name ?? 'عميل نقدي' }}</span></span>
      <span v-if="d.customer?.vatNumber">{{ d.profile.taxId.label }}: <span class="num text-doc-ink">{{ formatDigits(d.customer.vatNumber) }}</span></span>
      <span>الدفع: <span class="text-doc-ink">{{ d.paymentLabel }}</span></span>
      <span>الكاشير: <span class="text-doc-ink">{{ d.cashier }}</span></span>
      <span v-if="d.store.vatNumber" class="ms-auto">{{ d.profile.taxId.label }} البائع: <span class="num text-doc-ink">{{ formatDigits(d.store.vatNumber) }}</span></span>
    </div>

    <table class="mt-2 w-full border-collapse">
      <thead>
        <tr class="bg-doc-ink text-doc-paper">
          <th class="w-7 px-1.5 py-1 text-center font-medium">#</th>
          <th class="px-1.5 py-1 text-start font-medium">الصنف</th>
          <th class="px-1.5 py-1 text-center font-medium">الكمية</th>
          <th class="px-1.5 py-1 text-center font-medium">السعر</th>
          <th class="px-1.5 py-1 text-center font-medium">الخصم</th>
          <th class="px-1.5 py-1 text-center font-medium">القيمة</th>
          <th class="px-1.5 py-1 text-center font-medium">الضريبة</th>
          <th class="px-1.5 py-1 text-end font-medium">الإجمالي</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="l in d.lines" :key="l.id" class="odd:bg-doc-paper even:bg-print-fill">
          <td class="px-1.5 py-1 text-center text-print-muted"><span class="num">{{ formatNumber(l.no) }}</span></td>
          <td class="px-1.5 py-1">{{ l.name }}</td>
          <td class="px-1.5 py-1 text-center"><span class="num">{{ formatNumber(l.qty) }}</span></td>
          <td class="px-1.5 py-1 text-center"><MoneyText :value="l.price" plain /></td>
          <td class="px-1.5 py-1 text-center"><MoneyText :value="l.discount" plain dash-zero /></td>
          <td class="px-1.5 py-1 text-center"><MoneyText :value="l.amount" plain /></td>
          <td class="px-1.5 py-1 text-center"><MoneyText :value="l.vat" plain /></td>
          <td class="px-1.5 py-1 text-end font-medium"><MoneyText :value="l.total" plain /></td>
        </tr>
      </tbody>
      <tfoot>
        <tr class="border-t border-doc-ink text-print-muted">
          <td colspan="2" class="px-1.5 py-1">عدد الأصناف <span class="num text-doc-ink">{{ formatNumber(d.lines.length) }}</span></td>
          <td class="px-1.5 py-1 text-center"><span class="num text-doc-ink">{{ formatNumber(d.itemCount) }}</span></td>
          <td colspan="5" />
        </tr>
      </tfoot>
    </table>

    <!-- Totals strip -->
    <dl class="mt-3 flex border border-doc-ink">
      <div v-for="r in d.totals" :key="r.key" class="flex-1 border-e border-doc-ink px-2 py-1.5 last:border-e-0" :class="r.key === 'grand' ? 'flex-[1.4] bg-doc-ink text-doc-paper' : ''">
        <dt :class="r.key === 'grand' ? 'opacity-80' : 'text-print-muted'">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
        <dd class="font-semibold" :class="r.key === 'grand' ? 'text-lead' : 'text-body'"><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
      </div>
    </dl>
    <p class="mt-1 text-print-muted">{{ d.amountInWords }}</p>

    <footer class="mt-auto flex items-end justify-between gap-4 border-t border-print-rule pt-2 text-print-muted">
      <div>
        <p v-if="d.note">ملاحظات: {{ d.note }}</p>
        <p v-if="d.footer">{{ d.footer }}</p>
        <p>
          <template v-if="d.storeAddress">{{ d.storeAddress }}</template>
          <template v-if="d.store.phone"> — <span class="num" dir="ltr">{{ formatDigits(d.store.phone) }}</span></template>
        </p>
        <p v-if="d.sample" class="font-medium text-doc-ink">— نموذج اختبار طباعة، ليست فاتورة حقيقية —</p>
      </div>
      <QrCode v-if="d.qr" :value="d.qr" size="20mm" :border="0" />
    </footer>
  </article>
</template>
