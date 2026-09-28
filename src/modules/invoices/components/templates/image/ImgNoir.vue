<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';

/** Plan 22 · داكن فاخر — a dark card with a gold hairline frame, Naskh headings and dotted leaders, like a restaurant bill. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="w-[400px] bg-doc-noir p-4 text-label text-doc-paper" dir="rtl">
    <div class="border border-doc-gold/50 p-1">
      <div class="border border-doc-gold/20 px-6 py-7">
        <header class="text-center">
          <img v-if="d.store.logo" :src="d.store.logo" alt="" class="mx-auto mb-3 size-12 rounded-full bg-doc-paper object-contain p-1" />
          <p class="text-heading font-semibold tracking-wide text-doc-gold">{{ d.store.storeName }}</p>
          <div class="mx-auto mt-3 flex w-28 items-center gap-2" aria-hidden="true">
            <span class="h-px flex-1 bg-doc-gold/60" />
            <span class="size-1 rotate-45 bg-doc-gold" />
            <span class="h-px flex-1 bg-doc-gold/60" />
          </div>
          <p class="mt-3 text-tiny opacity-60">{{ d.title }}</p>
          <p class="num text-center text-tiny opacity-60">{{ d.number }} — {{ d.dateTime }}</p>
        </header>

        <ul class="mt-6 space-y-3">
          <li v-for="l in d.lines" :key="l.id">
            <div class="flex items-baseline gap-2">
              <span class="text-body">{{ l.name }}</span>
              <span class="flex-1 border-b border-dotted border-doc-gold/40" aria-hidden="true" />
              <MoneyText :value="l.amount" plain class="text-body text-doc-gold" />
            </div>
            <p class="num text-tiny opacity-50">{{ formatNumber(l.qty) }} × {{ formatNumber(l.price, 2) }}</p>
          </li>
        </ul>

        <dl class="mt-6 space-y-1 border-t border-doc-gold/25 pt-3 text-tiny">
          <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between opacity-80">
            <dt>{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
            <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
          </div>
        </dl>

        <div class="mt-5 bg-doc-noir-soft px-4 py-4 text-center">
          <p class="text-tiny opacity-60">الإجمالي</p>
          <p class="text-display font-semibold leading-tight text-doc-gold"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></p>
          <p class="mt-1 text-tiny opacity-60">{{ d.paymentLabel }}</p>
        </div>

        <footer class="mt-6 text-center text-tiny opacity-60">
          <p v-if="d.customer">مع التقدير لـ {{ d.customer.name }}</p>
          <p>{{ d.sample ? 'نموذج تجريبي' : d.footer || 'نتشرف بزيارتكم' }}</p>
        </footer>
      </div>
    </div>
  </article>
</template>
