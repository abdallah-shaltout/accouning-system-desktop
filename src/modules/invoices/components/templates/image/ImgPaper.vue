<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import DocBarcode from '../DocBarcode.vue';

/** Plan 22 · ورقة إيصال — a paper till roll with scalloped torn edges, photographed on a mustard desk. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));

// Scallops punched out of the paper's top/bottom edge in the desk colour.
const EDGE_TOP = { background: 'radial-gradient(circle at 50% 0, var(--color-doc-mustard) 6px, var(--color-doc-paper) 6.5px) 0 0 / 16px 10px repeat-x' };
const EDGE_BOTTOM = { background: 'radial-gradient(circle at 50% 100%, var(--color-doc-mustard) 6px, var(--color-doc-paper) 6.5px) 0 0 / 16px 10px repeat-x' };
</script>

<template>
  <article class="w-[400px] bg-doc-mustard px-7 py-8 text-label text-doc-ink" dir="rtl">
    <div class="-rotate-1 drop-shadow-md">
      <div class="h-2.5" :style="EDGE_TOP" aria-hidden="true" />
      <div class="bg-doc-paper px-6 pb-4 pt-3">
        <header class="text-center">
          <p class="text-lead font-semibold">{{ d.store.storeName }}</p>
          <p v-if="d.storeAddress" class="text-tiny text-print-muted">{{ d.storeAddress }}</p>
          <p v-if="d.store.phone" class="num text-center text-tiny text-print-muted" dir="ltr">{{ formatDigits(d.store.phone) }}</p>
        </header>

        <div class="my-3 border-t-2 border-dotted border-doc-ink/40" />
        <p class="text-center font-medium">{{ d.title }}</p>
        <dl class="mt-1 space-y-0.5 text-tiny">
          <div class="flex justify-between"><dt>رقم</dt><dd class="num">{{ d.number }}</dd></div>
          <div class="flex justify-between"><dt>التاريخ</dt><dd class="num">{{ d.dateTime }}</dd></div>
          <div class="flex justify-between"><dt>العميل</dt><dd>{{ d.customer?.name ?? 'عميل نقدي' }}</dd></div>
        </dl>

        <div class="my-3 border-t-2 border-dotted border-doc-ink/40" />
        <ul class="space-y-1.5">
          <li v-for="l in d.lines" :key="l.id">
            <p>{{ l.name }}</p>
            <div class="flex items-baseline gap-2 text-tiny">
              <span class="num text-print-muted">{{ formatNumber(l.qty) }} × {{ formatNumber(l.price, 2) }}</span>
              <span class="flex-1 border-b border-dotted border-doc-ink/30" aria-hidden="true" />
              <MoneyText :value="l.amount" plain class="text-label" />
            </div>
          </li>
        </ul>

        <div class="my-3 border-t-2 border-dotted border-doc-ink/40" />
        <dl class="space-y-0.5 text-tiny">
          <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between">
            <dt>{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
            <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" plain /></dd>
          </div>
        </dl>
        <div class="mt-2 flex items-baseline justify-between border-y-2 border-doc-ink py-1.5">
          <span class="text-body font-semibold">الإجمالي</span>
          <span class="text-heading font-semibold"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></span>
        </div>
        <p class="mt-1 text-center text-tiny">{{ d.paymentLabel }}<template v-if="d.change > 0">، الباقي <MoneyText :value="d.change" plain /></template></p>

        <DocBarcode :value="d.inv.number" class="mx-auto mt-4 h-10 w-4/5" />
        <p class="mt-3 text-center text-tiny">{{ d.sample ? 'نموذج تجريبي' : d.footer || 'شكراً لزيارتكم' }}</p>
      </div>
      <div class="h-2.5" :style="EDGE_BOTTOM" aria-hidden="true" />
    </div>
  </article>
</template>
