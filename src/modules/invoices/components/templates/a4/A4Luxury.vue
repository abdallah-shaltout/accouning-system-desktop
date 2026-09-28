<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import QrCode from '../../QrCode.vue';

/** Plan 22 · فاخر — a double bronze frame, a monogram medallion and a centred Naskh composition, like a boutique's stationery. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="min-h-[296mm] w-[210mm] bg-doc-paper p-[8mm] text-label leading-loose text-doc-ink" dir="rtl">
    <div class="flex min-h-[280mm] flex-col border-4 border-double border-doc-bronze px-[14mm] py-[12mm]">
      <header class="text-center">
        <div class="mx-auto grid size-20 place-items-center rounded-full border border-doc-bronze p-1">
          <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-full rounded-full object-contain" />
          <span v-else class="grid size-full place-items-center rounded-full bg-doc-bronze-soft text-heading font-semibold text-doc-bronze">{{ d.monogram }}</span>
        </div>
        <h1 class="mt-3 text-heading font-semibold tracking-wide">{{ d.store.storeName }}</h1>
        <p v-if="d.storeAddress" class="text-tiny text-print-muted">{{ d.storeAddress }}</p>

        <!-- Ornamental rule -->
        <div class="mx-auto mt-4 flex w-48 items-center gap-2" aria-hidden="true">
          <span class="h-px flex-1 bg-doc-bronze" />
          <span class="size-1.5 rotate-45 bg-doc-bronze" />
          <span class="h-px flex-1 bg-doc-bronze" />
        </div>
        <p class="mt-3 text-heading-sm font-medium text-doc-bronze">{{ d.title }}</p>
        <p v-if="d.titleEn" class="text-caption tracking-wide text-print-muted" dir="ltr">{{ d.titleEn }}</p>
      </header>

      <dl class="mx-auto mt-6 flex divide-x divide-doc-bronze/40 text-center text-tiny">
        <div class="px-6"><dt class="text-print-muted">رقم الفاتورة</dt><dd class="num text-center text-body font-medium">{{ d.number }}</dd></div>
        <div class="px-6"><dt class="text-print-muted">التاريخ</dt><dd class="num text-center text-body">{{ d.dateLong }}</dd></div>
        <div class="px-6"><dt class="text-print-muted">طريقة الدفع</dt><dd class="text-body">{{ d.paymentLabel }}</dd></div>
      </dl>

      <section class="mt-6 grid grid-cols-2 gap-8 text-center text-tiny">
        <div class="bg-doc-bronze-soft px-4 py-3">
          <p class="text-doc-bronze">مُقدَّمة من</p>
          <p class="text-body font-medium">{{ d.store.storeName }}</p>
          <p v-if="d.store.vatNumber">{{ d.profile.taxId.label }}: <span class="num">{{ formatDigits(d.store.vatNumber) }}</span></p>
          <p v-if="d.store.phone" class="num text-center" dir="ltr">{{ formatDigits(d.store.phone) }}</p>
        </div>
        <div class="bg-doc-bronze-soft px-4 py-3">
          <p class="text-doc-bronze">مُقدَّمة إلى</p>
          <template v-if="d.customer">
            <p class="text-body font-medium">{{ d.customer.name }}</p>
            <p v-if="d.customer.vatNumber">{{ d.profile.taxId.label }}: <span class="num">{{ formatDigits(d.customer.vatNumber) }}</span></p>
            <p v-if="d.customerAddress">{{ d.customerAddress }}</p>
          </template>
          <p v-else class="text-body font-medium">ضيفنا الكريم</p>
        </div>
      </section>

      <table class="mt-8 w-full text-body">
        <thead>
          <tr class="border-y border-doc-bronze text-tiny text-doc-bronze">
            <th class="py-2 text-start font-medium">الصنف</th>
            <th class="py-2 text-center font-medium">الكمية</th>
            <th class="py-2 text-center font-medium">السعر</th>
            <th class="py-2 text-end font-medium">القيمة</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="l in d.lines" :key="l.id" class="border-b border-doc-bronze/25">
            <td class="py-2.5">{{ l.name }}</td>
            <td class="py-2.5 text-center"><span class="num">{{ formatNumber(l.qty) }}</span></td>
            <td class="py-2.5 text-center"><MoneyText :value="l.price" plain /></td>
            <td class="py-2.5 text-end"><MoneyText :value="l.amount" plain /></td>
          </tr>
        </tbody>
      </table>

      <div class="mt-6 flex items-start justify-between gap-8">
        <QrCode v-if="d.qr" :value="d.qr" size="26mm" :border="1" />
        <span v-else />
        <dl class="w-[78mm] text-body">
          <div v-for="r in d.breakdown" :key="r.key" class="flex justify-between py-0.5">
            <dt class="text-print-muted">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
            <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
          </div>
          <div class="mt-2 flex justify-between border-y-4 border-double border-doc-bronze py-2 text-lead font-semibold">
            <dt>الإجمالي</dt>
            <dd class="text-doc-bronze"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></dd>
          </div>
        </dl>
      </div>
      <p class="mt-3 text-center text-tiny text-print-muted">{{ d.amountInWords }}</p>

      <footer class="mt-auto pt-10 text-center">
        <div class="mx-auto w-56 border-t border-doc-bronze pt-1 text-tiny text-print-muted">التوقيع</div>
        <p v-if="d.footer" class="mt-6 text-body text-doc-bronze">{{ d.footer }}</p>
        <p v-if="d.note" class="text-tiny text-print-muted">{{ d.note }}</p>
        <p v-if="d.sample" class="mt-1 text-tiny font-medium">— نموذج اختبار طباعة، ليست فاتورة حقيقية —</p>
      </footer>
    </div>
  </article>
</template>
