<script setup lang="ts">
import { reactive, toRef } from 'vue';
import { CheckCheck } from '@lucide/vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';

/** Plan 22 · محادثة — the invoice told as a short conversation: the store greets, lists the items as messages, sends the total; the payment comes back as a reply. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="w-[400px] bg-doc-wallpaper text-body text-doc-ink" dir="rtl">
    <header class="flex items-center gap-3 bg-doc-chat px-4 py-3 text-doc-paper">
      <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-10 rounded-full bg-doc-paper object-contain p-0.5" />
      <span v-else class="grid size-10 place-items-center rounded-full bg-doc-paper/20 text-body font-semibold">{{ d.monogram }}</span>
      <div class="min-w-0">
        <p class="truncate text-body font-medium">{{ d.store.storeName }}</p>
        <p class="num text-tiny opacity-75">{{ d.title }} {{ d.number }}</p>
      </div>
    </header>

    <div class="space-y-2 px-3 py-4 text-label">
      <p class="mx-auto w-fit rounded-lg bg-doc-paper/70 px-3 py-0.5 text-tiny text-print-muted">{{ d.dateLong }}</p>

      <!-- Store messages sit at the start edge -->
      <div class="me-12 w-fit rounded-2xl rounded-ss-sm bg-doc-paper px-3 py-2 shadow-sm">
        <p>أهلاً {{ d.customer?.name ?? 'بك' }}، هذه تفاصيل فاتورتك:</p>
        <p class="num mt-0.5 text-end text-caption text-print-muted">{{ d.time }}</p>
      </div>

      <div v-for="l in d.lines" :key="l.id" class="me-12 w-fit min-w-44 rounded-2xl rounded-ss-sm bg-doc-paper px-3 py-2 shadow-sm">
        <p class="font-medium">{{ l.name }}</p>
        <div class="mt-0.5 flex items-baseline justify-between gap-6">
          <span class="num text-tiny text-print-muted">{{ formatNumber(l.qty) }} × {{ formatNumber(l.price, 2) }}</span>
          <MoneyText :value="l.amount" :currency="d.currency" class="font-medium text-doc-chat" />
        </div>
      </div>

      <div class="me-12 w-fit min-w-56 rounded-2xl rounded-ss-sm bg-doc-paper px-3 py-2.5 shadow-sm">
        <dl class="space-y-0.5 text-tiny">
          <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between gap-6">
            <dt class="text-print-muted">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
            <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
          </div>
        </dl>
        <div class="mt-1.5 flex items-baseline justify-between gap-6 border-t border-print-rule pt-1.5">
          <span class="font-medium">الإجمالي</span>
          <span class="text-heading-sm font-semibold text-doc-chat"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></span>
        </div>
      </div>

      <!-- Customer reply sits at the end edge -->
      <div class="ms-auto w-fit rounded-2xl rounded-se-sm bg-doc-bubble px-3 py-2 shadow-sm">
        <p v-if="d.outstanding > 0">تمام، المتبقي <MoneyText :value="d.outstanding" :currency="d.currency" /> على حسابي</p>
        <p v-else>تم الدفع، {{ d.paymentLabel }}</p>
        <p class="mt-0.5 flex items-center justify-end gap-1 text-caption text-print-muted">
          <span class="num">{{ d.time }}</span>
          <CheckCheck class="size-3.5 text-doc-ios-blue" />
        </p>
      </div>

      <p v-if="d.footer || d.sample" class="mx-auto w-fit rounded-lg bg-doc-paper/70 px-3 py-0.5 text-tiny text-print-muted">{{ d.sample ? 'نموذج تجريبي' : d.footer }}</p>
    </div>
  </article>
</template>
