<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';

/** Plan 22 · إجمالي بارز — almost nothing but the number: a huge centred total, a coral status pill, and a quiet list underneath. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="w-[400px] bg-doc-paper px-7 pb-8 pt-7 text-label text-doc-ink" dir="rtl">
    <header class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-8 rounded-full object-contain" />
        <span class="font-semibold">{{ d.store.storeName }}</span>
      </div>
      <span class="num text-tiny text-print-muted">{{ d.date }}</span>
    </header>

    <section class="py-12 text-center">
      <p class="text-tiny text-print-muted">{{ d.customer ? `فاتورة ${d.customer.name}` : d.title }}</p>
      <p class="mt-2 font-semibold leading-none tracking-tight [font-size:calc(var(--text-display)*1.35)]">
        <MoneyText :value="d.inv.grandTotal" :currency="d.currency" />
      </p>
      <span class="mt-4 inline-flex items-center gap-1.5 rounded-full bg-doc-coral-soft px-3 py-1 text-tiny font-medium text-doc-coral">
        <span class="size-1.5 rounded-full bg-doc-coral" aria-hidden="true" />
        {{ d.paymentStatus.label }}، {{ d.paymentLabel }}
      </span>
    </section>

    <ul class="space-y-2.5 border-t border-print-rule pt-5">
      <li v-for="l in d.lines" :key="l.id" class="flex justify-between gap-4">
        <span class="text-print-muted"><span class="num text-doc-coral">{{ formatNumber(l.qty) }}</span> {{ l.name }}</span>
        <MoneyText :value="l.amount" plain />
      </li>
    </ul>

    <dl class="mt-5 space-y-1 border-t border-print-rule pt-4 text-tiny text-print-muted">
      <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between">
        <dt>{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
        <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
      </div>
    </dl>

    <footer class="mt-8 text-center text-tiny text-print-muted">
      <p class="num text-center">{{ d.number }}</p>
      <p>{{ d.sample ? 'نموذج تجريبي' : d.footer }}</p>
    </footer>
  </article>
</template>
