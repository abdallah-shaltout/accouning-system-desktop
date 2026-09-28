<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import QrCode from '../../QrCode.vue';

/** Plan 22 · سويسري — typography does the work: an oversized invoice number, a label column, hairlines only and a single red mark. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="flex min-h-[296mm] w-[210mm] flex-col bg-doc-paper px-[16mm] py-[16mm] text-label leading-relaxed text-doc-ink" dir="rtl">
    <header class="flex items-start justify-between">
      <div class="flex items-center gap-2.5">
        <span class="size-3.5 bg-doc-red" aria-hidden="true" />
        <span class="text-lead font-semibold">{{ d.store.storeName }}</span>
      </div>
      <span class="num text-body">{{ d.dateLong }}</span>
    </header>

    <section class="mt-16">
      <p class="text-body text-print-muted">{{ d.title }}</p>
      <p class="num font-semibold leading-none tracking-tight [font-size:calc(var(--text-display)*1.7)]">{{ d.number }}</p>
    </section>

    <div class="mt-14 grid grid-cols-[42mm_1fr] text-body">
      <p class="border-t border-doc-ink pt-2 text-print-muted">إلى</p>
      <div class="border-t border-doc-ink pb-6 pt-2">
        <template v-if="d.customer">
          <p class="font-medium">{{ d.customer.name }}</p>
          <p v-if="d.customerAddress" class="text-print-muted">{{ d.customerAddress }}</p>
          <p v-if="d.customer.vatNumber" class="num text-print-muted">{{ formatDigits(d.customer.vatNumber) }}</p>
        </template>
        <p v-else class="font-medium">عميل نقدي</p>
      </div>

      <p class="border-t border-doc-ink pt-2 text-print-muted">من</p>
      <div class="grid grid-cols-2 gap-6 border-t border-doc-ink pb-6 pt-2">
        <div>
          <p class="font-medium">{{ d.store.storeName }}</p>
          <p v-if="d.storeAddress" class="text-print-muted">{{ d.storeAddress }}</p>
        </div>
        <div class="text-print-muted">
          <p v-if="d.store.phone" class="num" dir="ltr">{{ formatDigits(d.store.phone) }}</p>
          <p v-if="d.store.vatNumber">{{ d.profile.taxId.label }} <span class="num text-doc-ink">{{ formatDigits(d.store.vatNumber) }}</span></p>
        </div>
      </div>

      <p class="border-t border-doc-ink pt-2 text-print-muted">الأصناف</p>
      <ol class="border-t border-doc-ink pb-6">
        <li v-for="l in d.lines" :key="l.id" class="grid grid-cols-[8mm_1fr_auto_28mm] items-baseline gap-4 border-b border-print-rule py-2.5">
          <span class="num text-tiny text-print-muted">{{ String(l.no).padStart(2, '0') }}</span>
          <span>{{ l.name }}</span>
          <span class="num text-print-muted">{{ formatNumber(l.qty) }} × {{ formatNumber(l.price, 2) }}</span>
          <span class="text-end"><MoneyText :value="l.amount" plain /></span>
        </li>
      </ol>

      <p class="border-t border-doc-ink pt-2 text-print-muted">المبالغ</p>
      <dl class="border-t border-doc-ink pt-2">
        <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between py-1">
          <dt class="text-print-muted">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
          <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
        </div>
      </dl>
    </div>

    <section class="mt-10 grid grid-cols-[42mm_1fr] items-end">
      <div>
        <QrCode v-if="d.qr" :value="d.qr" size="26mm" :border="0" />
      </div>
      <div class="border-t-4 border-doc-red pt-3">
        <div class="flex items-end justify-between gap-6">
          <span class="text-body">الإجمالي<br /><span class="text-tiny text-print-muted">{{ d.paymentLabel }}</span></span>
          <span class="text-display font-semibold leading-none tracking-tight"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></span>
        </div>
        <p class="mt-2 text-tiny text-print-muted">{{ d.amountInWords }}</p>
      </div>
    </section>

    <footer class="mt-auto grid grid-cols-[42mm_1fr] pt-10 text-tiny text-print-muted">
      <span>{{ d.cashier }}</span>
      <div>
        <p v-if="d.note">{{ d.note }}</p>
        <p v-if="d.footer">{{ d.footer }}</p>
        <p v-if="d.sample" class="font-medium text-doc-ink">— نموذج اختبار طباعة، ليست فاتورة حقيقية —</p>
      </div>
    </footer>
  </article>
</template>
