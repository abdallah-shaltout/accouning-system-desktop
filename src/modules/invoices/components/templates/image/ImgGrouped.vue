<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';

/** Plan 22 · قوائم مجمعة — laid out like a phone's settings screen: a large title, then grouped rounded lists with inset dividers and grey section captions. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="w-[400px] bg-doc-ios px-4 pb-6 pt-8 text-body text-doc-ink" dir="rtl">
    <p class="px-2 text-tiny text-print-muted">{{ d.store.storeName }}</p>
    <h1 class="px-2 text-stat font-semibold leading-tight">{{ d.title }}</h1>
    <p class="num px-2 text-label text-print-muted">{{ d.number }}</p>

    <p class="mt-6 px-4 pb-1.5 text-tiny text-print-muted">التفاصيل</p>
    <dl class="rounded-2xl bg-doc-paper">
      <div class="flex justify-between px-4 py-2.5"><dt>العميل</dt><dd class="text-print-muted">{{ d.customer?.name ?? 'عميل نقدي' }}</dd></div>
      <div class="ms-4 flex justify-between border-t border-print-rule py-2.5 pe-4"><dt>التاريخ</dt><dd class="num text-print-muted">{{ d.dateTime }}</dd></div>
      <div class="ms-4 flex justify-between border-t border-print-rule py-2.5 pe-4"><dt>طريقة الدفع</dt><dd class="text-print-muted">{{ d.paymentLabel }}</dd></div>
      <div class="ms-4 flex justify-between border-t border-print-rule py-2.5 pe-4"><dt>الحالة</dt><dd class="text-doc-ios-blue">{{ d.paymentStatus.label }}</dd></div>
    </dl>

    <p class="mt-6 px-4 pb-1.5 text-tiny text-print-muted">الأصناف</p>
    <ul class="rounded-2xl bg-doc-paper">
      <li v-for="(l, i) in d.lines" :key="l.id" class="flex items-center justify-between gap-3 py-2.5 pe-4" :class="i === 0 ? 'ps-4' : 'ms-4 border-t border-print-rule'">
        <span class="min-w-0">
          <span class="block truncate">{{ l.name }}</span>
          <span class="num block text-tiny text-print-muted">{{ formatNumber(l.qty) }} × {{ formatNumber(l.price, 2) }}</span>
        </span>
        <MoneyText :value="l.amount" plain class="text-print-muted" />
      </li>
    </ul>

    <p class="mt-6 px-4 pb-1.5 text-tiny text-print-muted">الملخص</p>
    <dl class="rounded-2xl bg-doc-paper">
      <div v-for="(r, i) in d.totals" :key="r.key" class="flex justify-between py-2.5 pe-4" :class="[i === 0 ? 'ps-4' : 'ms-4 border-t border-print-rule', r.key === 'grand' ? 'text-lead font-semibold text-doc-ios-blue' : '']">
        <dt>{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
        <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
      </div>
    </dl>
    <p class="px-4 pt-1.5 text-tiny text-print-muted">{{ d.amountInWords }}</p>

    <p class="mt-6 px-4 text-center text-tiny text-print-muted">
      <template v-if="d.store.phone"><span class="num" dir="ltr">{{ formatDigits(d.store.phone) }}</span><br /></template>
      {{ d.sample ? 'نموذج تجريبي' : d.footer }}
    </p>
  </article>
</template>
