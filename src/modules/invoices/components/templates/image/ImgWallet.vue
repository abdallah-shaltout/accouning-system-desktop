<script setup lang="ts">
import { reactive, toRef } from 'vue';
import { Wifi } from '@lucide/vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';

/** Plan 22 · بطاقة بنكية — the total sits on a dark bank card (chip, grouped number, holder name); the items read like a statement below it. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="w-[400px] bg-doc-paper px-5 pb-6 pt-5 text-body text-doc-ink" dir="rtl">
    <div class="flex items-center justify-between text-label">
      <span class="font-semibold">{{ d.store.storeName }}</span>
      <span class="num text-print-muted">{{ d.dateTime }}</span>
    </div>

    <!-- Card -->
    <div class="relative mt-4 aspect-[1.586] overflow-hidden rounded-3xl bg-linear-to-br from-doc-forest to-doc-jade p-5 text-doc-paper shadow-xl">
      <div class="absolute -end-16 -top-20 size-56 rounded-full bg-doc-paper/5" aria-hidden="true" />
      <div class="absolute -bottom-24 -start-10 size-56 rounded-full bg-doc-lime/10" aria-hidden="true" />
      <div class="relative flex h-full flex-col">
        <div class="flex items-start justify-between">
          <span class="h-8 w-11 rounded-md bg-doc-lime/90" aria-hidden="true" />
          <Wifi class="size-5 rotate-90 opacity-70" aria-hidden="true" />
        </div>
        <p class="mt-auto text-tiny opacity-70">إجمالي الفاتورة</p>
        <p class="text-stat font-semibold leading-tight"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></p>
        <div class="mt-3 flex items-end justify-between text-tiny">
          <span class="opacity-80">{{ d.customer?.name ?? 'عميل نقدي' }}</span>
          <span class="num tracking-widest opacity-80" dir="ltr">•••• {{ d.inv.number.slice(-4) }}</span>
        </div>
      </div>
    </div>

    <div class="mt-4 grid grid-cols-2 gap-2 text-tiny">
      <div class="rounded-2xl bg-print-fill px-3 py-2"><p class="text-print-muted">الدفع</p><p class="text-label font-medium">{{ d.paymentLabel }}</p></div>
      <div class="rounded-2xl bg-print-fill px-3 py-2"><p class="text-print-muted">رقم الفاتورة</p><p class="num text-label font-medium">{{ d.number }}</p></div>
    </div>

    <p class="mt-5 text-label font-medium">الأصناف</p>
    <ul class="mt-2 divide-y divide-print-rule">
      <li v-for="l in d.lines" :key="l.id" class="flex items-center gap-3 py-2.5">
        <span class="num grid size-9 shrink-0 place-items-center rounded-full bg-doc-jade/10 text-tiny font-semibold text-doc-jade">{{ formatNumber(l.qty) }}×</span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-label font-medium">{{ l.name }}</span>
          <span class="num block text-tiny text-print-muted">{{ formatNumber(l.price, 2) }} للوحدة</span>
        </span>
        <MoneyText :value="l.amount" plain class="text-label font-medium" />
      </li>
    </ul>

    <dl class="mt-2 space-y-1 rounded-2xl bg-print-fill px-4 py-3 text-label">
      <div v-for="r in d.totals" :key="r.key" class="flex justify-between" :class="r.key === 'grand' ? 'border-t border-print-rule pt-1.5 font-semibold text-doc-forest' : ''">
        <dt :class="r.key === 'grand' ? '' : 'text-print-muted'">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
        <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
      </div>
    </dl>

    <p v-if="d.footer || d.sample" class="mt-4 text-center text-tiny text-print-muted">{{ d.sample ? 'نموذج تجريبي' : d.footer }}</p>
  </article>
</template>
