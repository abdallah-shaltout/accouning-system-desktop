<script setup lang="ts">
import { reactive, toRef } from 'vue';
import { ShoppingBag } from '@lucide/vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import DocBarcode from '../DocBarcode.vue';

/** Plan 22 · تذكرة — a boarding pass: store and customer as the two "stations", a perforated tear line, and a stub with the items, the total and a scannable barcode. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="w-[400px] bg-doc-cobalt-soft p-5 text-body text-doc-ink" dir="rtl">
    <div class="rounded-3xl bg-doc-paper shadow-sm">
      <header class="rounded-t-3xl bg-doc-cobalt px-6 pb-6 pt-5 text-doc-paper">
        <div class="flex items-center justify-between text-tiny opacity-80">
          <span>{{ d.title }}</span>
          <span class="num">{{ d.number }}</span>
        </div>
        <div class="mt-4 grid grid-cols-[1fr_auto_1fr] items-center gap-3">
          <div>
            <p class="text-display font-semibold leading-none">{{ d.monogram }}</p>
            <p class="mt-1 truncate text-tiny opacity-80">{{ d.store.storeName }}</p>
          </div>
          <div class="flex w-24 items-center gap-1.5 opacity-80" aria-hidden="true">
            <span class="h-px flex-1 border-t border-dashed border-doc-paper" />
            <ShoppingBag class="size-4" />
            <span class="h-px flex-1 border-t border-dashed border-doc-paper" />
          </div>
          <div class="text-end">
            <p class="text-display font-semibold leading-none">{{ d.customer ? d.customer.name.trim().charAt(0) : 'ع' }}</p>
            <p class="mt-1 truncate text-tiny opacity-80">{{ d.customer?.name ?? 'عميل نقدي' }}</p>
          </div>
        </div>
      </header>

      <dl class="grid grid-cols-3 gap-y-3 px-6 py-5 text-tiny">
        <div><dt class="text-print-muted">التاريخ</dt><dd class="num text-label font-medium">{{ d.date }}</dd></div>
        <div><dt class="text-print-muted">الوقت</dt><dd class="num text-label font-medium">{{ d.time }}</dd></div>
        <div><dt class="text-print-muted">الأصناف</dt><dd class="num text-label font-medium">{{ formatNumber(d.itemCount) }}</dd></div>
        <div><dt class="text-print-muted">الدفع</dt><dd class="text-label font-medium">{{ d.paymentLabel }}</dd></div>
        <div><dt class="text-print-muted">الكاشير</dt><dd class="truncate text-label font-medium">{{ d.cashier }}</dd></div>
        <div><dt class="text-print-muted">الحالة</dt><dd class="text-label font-medium text-doc-cobalt">{{ d.paymentStatus.label }}</dd></div>
      </dl>

      <!-- Tear line -->
      <div class="relative mx-6 border-t-2 border-dashed border-doc-cobalt/25" aria-hidden="true">
        <span class="absolute -start-9 -top-3 size-6 rounded-full bg-doc-cobalt-soft" />
        <span class="absolute -end-9 -top-3 size-6 rounded-full bg-doc-cobalt-soft" />
      </div>

      <section class="px-6 pb-6 pt-5">
        <ul class="space-y-1.5 text-label">
          <li v-for="l in d.lines" :key="l.id" class="flex justify-between gap-3">
            <span class="truncate"><span class="num text-print-muted">{{ formatNumber(l.qty) }}×</span> {{ l.name }}</span>
            <MoneyText :value="l.amount" plain />
          </li>
        </ul>
        <dl class="mt-3 space-y-0.5 border-t border-print-rule pt-2 text-tiny">
          <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between">
            <dt class="text-print-muted">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
            <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
          </div>
        </dl>
        <div class="mt-3 flex items-end justify-between">
          <span class="text-label text-print-muted">الإجمالي</span>
          <span class="text-stat font-semibold leading-none text-doc-cobalt"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></span>
        </div>
        <DocBarcode :value="d.inv.number" class="mt-5 h-12" />
        <p class="num mt-1 text-center text-caption tracking-widest text-print-muted">{{ d.inv.number }}</p>
        <p v-if="d.footer || d.sample" class="mt-3 text-center text-tiny text-print-muted">{{ d.sample ? 'نموذج تجريبي' : d.footer }}</p>
      </section>
    </div>
  </article>
</template>
