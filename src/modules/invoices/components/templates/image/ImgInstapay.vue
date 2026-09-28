<script setup lang="ts">
import { reactive, toRef } from 'vue';
import { Check } from '@lucide/vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';

/** Plan 22 · إيصال تحويل — in the style of an InstaPay transfer receipt: violet gradient, success seal, one big amount, then detail rows. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="w-[400px] bg-doc-paper pb-6 text-body text-doc-ink" dir="rtl">
    <header class="relative rounded-b-4xl bg-linear-to-bl from-doc-violet to-doc-magenta px-6 pb-16 pt-7 text-center text-doc-paper">
      <p class="text-label opacity-80">{{ d.store.storeName }}</p>
      <div class="mx-auto mt-5 grid size-16 place-items-center rounded-full bg-doc-paper/15 ring-8 ring-doc-paper/10">
        <span class="grid size-11 place-items-center rounded-full bg-doc-mint">
          <Check class="size-6" :stroke-width="3" />
        </span>
      </div>
      <p class="mt-4 text-lead font-medium">{{ d.outstanding > 0 ? 'تم تسجيل الفاتورة' : 'تمت العملية بنجاح' }}</p>
      <p class="mt-2 text-display font-semibold leading-none"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></p>
      <p class="num mt-3 text-center text-tiny opacity-80">{{ d.dateTime }}</p>
    </header>

    <section class="relative mx-5 -mt-10 rounded-3xl bg-doc-paper px-5 py-4 shadow-lg ring-1 ring-doc-violet/10">
      <dl class="divide-y divide-doc-violet/10 text-label">
        <div class="flex justify-between gap-4 py-2.5"><dt class="text-print-muted">رقم المرجع</dt><dd class="num font-medium">{{ d.number }}</dd></div>
        <div class="flex justify-between gap-4 py-2.5"><dt class="text-print-muted">من</dt><dd class="font-medium">{{ d.store.storeName }}</dd></div>
        <div class="flex justify-between gap-4 py-2.5"><dt class="text-print-muted">إلى</dt><dd class="font-medium">{{ d.customer?.name ?? 'عميل نقدي' }}</dd></div>
        <div class="flex justify-between gap-4 py-2.5"><dt class="text-print-muted">طريقة الدفع</dt><dd class="font-medium">{{ d.paymentLabel }}</dd></div>
        <div class="flex justify-between gap-4 py-2.5">
          <dt class="text-print-muted">الحالة</dt>
          <dd class="rounded-full bg-doc-mint/15 px-2.5 text-tiny font-medium text-doc-mint">{{ d.paymentStatus.label }}</dd>
        </div>
      </dl>
    </section>

    <section class="mx-5 mt-4 rounded-3xl bg-doc-violet/5 px-5 py-4">
      <p class="mb-2 text-label font-medium text-doc-violet">تفاصيل الأصناف <span class="num text-print-muted">({{ formatNumber(d.lines.length) }})</span></p>
      <ul class="space-y-2 text-label">
        <li v-for="l in d.lines" :key="l.id" class="flex items-start justify-between gap-3">
          <span>
            {{ l.name }}
            <span class="num block text-tiny text-print-muted">{{ formatNumber(l.qty) }} × {{ formatNumber(l.price, 2) }}</span>
          </span>
          <MoneyText :value="l.amount" plain class="font-medium" />
        </li>
      </ul>
      <div class="my-3 border-t border-dashed border-doc-violet/25" />
      <dl class="space-y-1 text-label">
        <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between">
          <dt class="text-print-muted">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
          <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
        </div>
      </dl>
    </section>

    <footer class="mt-5 px-6 text-center text-tiny text-print-muted">
      <p v-if="d.footer">{{ d.footer }}</p>
      <p v-if="d.sample" class="font-medium text-doc-ink">نموذج تجريبي</p>
    </footer>
  </article>
</template>
