<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';

/** Plan 22 · زجاجي — a soft gradient mesh behind a frosted card; the total floats on the colour above it. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));

const MESH = {
  backgroundColor: 'var(--color-doc-glass-a)',
  backgroundImage: [
    'radial-gradient(at 12% 8%, var(--color-doc-glass-c) 0, transparent 45%)',
    'radial-gradient(at 88% 22%, var(--color-doc-glass-b) 0, transparent 50%)',
    'radial-gradient(at 30% 90%, var(--color-doc-glass-b) 0, transparent 45%)',
  ].join(', '),
};
</script>

<template>
  <article class="w-[400px] px-5 pb-6 pt-8 text-body text-doc-ink" :style="MESH" dir="rtl">
    <header class="px-2 text-doc-paper">
      <div class="flex items-center gap-2">
        <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-9 rounded-xl bg-doc-paper/80 object-contain p-1" />
        <span class="text-body font-medium">{{ d.store.storeName }}</span>
      </div>
      <p class="mt-6 text-label opacity-85">{{ d.title }}</p>
      <p class="text-display font-semibold leading-tight"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></p>
      <p class="num text-tiny opacity-85">{{ d.number }} — {{ d.dateTime }}</p>
    </header>

    <section class="mt-6 rounded-3xl border border-doc-paper/60 bg-doc-paper/75 p-5 shadow-lg backdrop-blur-xl">
      <div class="grid grid-cols-2 gap-3 text-tiny">
        <div class="rounded-2xl bg-doc-paper/70 px-3 py-2">
          <p class="text-print-muted">العميل</p>
          <p class="truncate text-label font-medium">{{ d.customer?.name ?? 'عميل نقدي' }}</p>
        </div>
        <div class="rounded-2xl bg-doc-paper/70 px-3 py-2">
          <p class="text-print-muted">الدفع</p>
          <p class="text-label font-medium">{{ d.paymentLabel }}</p>
        </div>
      </div>

      <ul class="mt-4 space-y-2.5 text-label">
        <li v-for="l in d.lines" :key="l.id" class="flex items-center gap-3">
          <span class="num grid size-8 shrink-0 place-items-center rounded-xl bg-linear-to-br from-doc-glass-a to-doc-glass-b text-tiny font-semibold text-doc-paper">{{ formatNumber(l.qty) }}</span>
          <span class="min-w-0 flex-1">
            <span class="block truncate font-medium">{{ l.name }}</span>
            <span class="num block text-tiny text-print-muted">{{ formatNumber(l.price, 2) }} للوحدة</span>
          </span>
          <MoneyText :value="l.amount" plain class="font-medium" />
        </li>
      </ul>

      <dl class="mt-4 space-y-1 border-t border-doc-ink/10 pt-3 text-label">
        <div v-for="r in d.totals" :key="r.key" class="flex justify-between" :class="r.key === 'grand' ? 'pt-1 text-lead font-semibold' : ''">
          <dt :class="r.key === 'grand' ? '' : 'text-print-muted'">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
          <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
        </div>
      </dl>
    </section>

    <p class="mt-4 text-center text-tiny text-doc-paper">{{ d.sample ? 'نموذج تجريبي' : d.footer }}</p>
  </article>
</template>
