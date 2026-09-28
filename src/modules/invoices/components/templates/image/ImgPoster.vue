<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';

/** Plan 22 · ملصق — loud and graphic: a lime field, oversized black type, thick-ruled boxes and an inverted total block. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="w-[400px] bg-doc-poster p-6 text-body text-doc-ink" dir="rtl">
    <header class="flex items-start justify-between border-b-4 border-doc-ink pb-3">
      <p class="max-w-52 text-heading font-semibold leading-tight">{{ d.store.storeName }}</p>
      <p class="num rounded-full border-2 border-doc-ink px-3 py-0.5 text-label font-semibold">{{ d.date }}</p>
    </header>

    <p class="mt-5 font-semibold leading-none tracking-tight [font-size:calc(var(--text-display)*1.8)]">فاتورة</p>
    <p class="num mt-1 text-lead font-semibold">{{ d.number }}</p>

    <div class="mt-5 grid grid-cols-2 border-2 border-doc-ink text-label">
      <div class="border-e-2 border-doc-ink px-3 py-2">
        <p class="text-tiny">إلى</p>
        <p class="truncate font-semibold">{{ d.customer?.name ?? 'عميل نقدي' }}</p>
      </div>
      <div class="px-3 py-2">
        <p class="text-tiny">الدفع</p>
        <p class="font-semibold">{{ d.paymentLabel }}</p>
      </div>
    </div>

    <ul class="mt-4 border-2 border-doc-ink bg-doc-paper">
      <li v-for="l in d.lines" :key="l.id" class="flex items-center gap-3 border-b-2 border-doc-ink px-3 py-2 last:border-b-0">
        <span class="num grid size-8 shrink-0 place-items-center bg-doc-ink text-label font-semibold text-doc-poster">{{ formatNumber(l.qty) }}</span>
        <span class="min-w-0 flex-1 truncate font-medium">{{ l.name }}</span>
        <MoneyText :value="l.amount" plain class="font-semibold" />
      </li>
    </ul>

    <dl class="mt-4 space-y-0.5 text-label">
      <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between">
        <dt>{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
        <dd class="font-medium"><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
      </div>
    </dl>

    <div class="mt-3 bg-doc-ink px-4 py-4 text-doc-poster">
      <p class="text-label">الإجمالي</p>
      <p class="text-display font-semibold leading-none"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></p>
    </div>

    <p class="mt-4 text-center text-label font-medium">{{ d.sample ? 'نموذج تجريبي' : d.footer || 'شكراً!' }}</p>
  </article>
</template>
