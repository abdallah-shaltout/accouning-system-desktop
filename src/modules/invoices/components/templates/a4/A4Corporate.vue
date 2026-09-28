<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import QrCode from '../../QrCode.vue';

/** Plan 22 · مؤسسي — a government-form grid: boxed meta cells, a fully bordered table, stamp and signature boxes. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="flex min-h-[296mm] w-[210mm] flex-col bg-doc-paper px-[14mm] py-[12mm] text-label leading-relaxed text-doc-ink" dir="rtl">
    <!-- Header: seller · title box · logo -->
    <header class="grid grid-cols-[1fr_auto_1fr] items-center gap-6 border-b-4 border-double border-doc-navy pb-4">
      <div>
        <h1 class="text-heading font-semibold text-doc-navy">{{ d.store.storeName }}</h1>
        <p v-if="d.storeAddress" class="text-tiny text-print-muted">{{ d.storeAddress }}</p>
        <p v-if="d.store.phone" class="text-tiny text-print-muted">هاتف: <span class="num" dir="ltr">{{ formatDigits(d.store.phone) }}</span></p>
      </div>
      <div class="border-2 border-doc-navy px-6 py-2 text-center">
        <p class="text-heading-sm font-semibold text-doc-navy">{{ d.title }}</p>
        <p v-if="d.titleEn" class="text-caption tracking-wide text-print-muted" dir="ltr">{{ d.titleEn }}</p>
      </div>
      <div class="flex justify-end">
        <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-20 border border-print-rule object-contain p-1" />
        <div v-else class="grid size-20 place-items-center border-2 border-doc-navy text-heading font-semibold text-doc-navy">{{ d.monogram }}</div>
      </div>
    </header>

    <!-- Registration strip -->
    <div class="mt-3 flex flex-wrap gap-x-6 text-tiny">
      <span v-if="d.store.vatNumber">{{ d.profile.taxId.label }}: <span class="num font-medium">{{ formatDigits(d.store.vatNumber) }}</span></span>
      <span v-if="d.store.commercialRegister">{{ d.profile.commercialRegister.label }}: <span class="num font-medium">{{ formatDigits(d.store.commercialRegister) }}</span></span>
    </div>

    <!-- Meta cells -->
    <dl class="mt-3 grid grid-cols-4 border border-doc-navy text-tiny">
      <div class="border-e border-doc-navy px-3 py-2">
        <dt class="text-print-muted">رقم الفاتورة</dt>
        <dd class="num text-body font-semibold">{{ d.number }}</dd>
      </div>
      <div class="border-e border-doc-navy px-3 py-2">
        <dt class="text-print-muted">التاريخ</dt>
        <dd class="num text-body font-medium">{{ d.dateTime }}</dd>
      </div>
      <div class="border-e border-doc-navy px-3 py-2">
        <dt class="text-print-muted">طريقة الدفع</dt>
        <dd class="text-body font-medium">{{ d.paymentLabel }}</dd>
      </div>
      <div class="px-3 py-2">
        <dt class="text-print-muted">الكاشير</dt>
        <dd class="text-body font-medium">{{ d.cashier }}</dd>
      </div>
    </dl>

    <!-- Parties -->
    <section class="mt-4 grid grid-cols-2 gap-4 text-tiny">
      <div class="border border-doc-navy">
        <p class="bg-doc-navy px-3 py-1 font-medium text-doc-paper">بيانات البائع</p>
        <div class="space-y-0.5 px-3 py-2">
          <p class="text-label font-medium">{{ d.store.storeName }}</p>
          <p v-if="d.storeAddress" class="text-print-muted">{{ d.storeAddress }}</p>
          <p v-if="d.store.vatNumber">{{ d.profile.taxId.label }}: <span class="num">{{ formatDigits(d.store.vatNumber) }}</span></p>
        </div>
      </div>
      <div class="border border-doc-navy">
        <p class="bg-doc-navy px-3 py-1 font-medium text-doc-paper">بيانات المشتري</p>
        <div class="space-y-0.5 px-3 py-2">
          <template v-if="d.customer">
            <p class="text-label font-medium">{{ d.customer.name }}</p>
            <p v-if="d.customerAddress" class="text-print-muted">{{ d.customerAddress }}</p>
            <p v-if="d.customer.vatNumber">{{ d.profile.taxId.label }}: <span class="num">{{ formatDigits(d.customer.vatNumber) }}</span></p>
            <p v-if="d.customer.phone" class="num text-print-muted" dir="ltr">{{ formatDigits(d.customer.phone) }}</p>
          </template>
          <p v-else class="text-label">عميل نقدي</p>
        </div>
      </div>
    </section>

    <!-- Lines -->
    <table class="mt-5 w-full border-collapse text-tiny">
      <thead>
        <tr class="bg-doc-navy text-doc-paper">
          <th class="w-9 border border-doc-navy px-2 py-1.5 text-center font-medium">م</th>
          <th class="border border-doc-navy px-2 py-1.5 text-start font-medium">البيان</th>
          <th class="border border-doc-navy px-2 py-1.5 text-center font-medium">الكمية</th>
          <th class="border border-doc-navy px-2 py-1.5 text-center font-medium">سعر الوحدة</th>
          <th class="border border-doc-navy px-2 py-1.5 text-center font-medium">القيمة</th>
          <th class="border border-doc-navy px-2 py-1.5 text-center font-medium">الضريبة</th>
          <th class="border border-doc-navy px-2 py-1.5 text-center font-medium">الإجمالي</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="l in d.lines" :key="l.id" class="even:bg-doc-navy-soft">
          <td class="border border-doc-navy/40 px-2 py-1.5 text-center"><span class="num">{{ formatNumber(l.no) }}</span></td>
          <td class="border border-doc-navy/40 px-2 py-1.5">{{ l.name }}</td>
          <td class="border border-doc-navy/40 px-2 py-1.5 text-center"><span class="num">{{ formatNumber(l.qty) }}</span></td>
          <td class="border border-doc-navy/40 px-2 py-1.5 text-center"><MoneyText :value="l.price" plain /></td>
          <td class="border border-doc-navy/40 px-2 py-1.5 text-center"><MoneyText :value="l.amount" plain /></td>
          <td class="border border-doc-navy/40 px-2 py-1.5 text-center"><MoneyText :value="l.vat" plain /></td>
          <td class="border border-doc-navy/40 px-2 py-1.5 text-center font-medium"><MoneyText :value="l.total" plain /></td>
        </tr>
      </tbody>
    </table>

    <!-- Words + totals -->
    <section class="mt-4 grid grid-cols-[1fr_78mm] gap-4 text-tiny">
      <div class="space-y-3">
        <div class="border border-doc-navy px-3 py-2">
          <p class="text-print-muted">المبلغ بالحروف</p>
          <p class="text-label font-medium">{{ d.amountInWords }}</p>
        </div>
        <p v-if="d.note" class="border border-dashed border-doc-navy/50 px-3 py-2">ملاحظات: {{ d.note }}</p>
      </div>
      <table class="w-full border-collapse">
        <tbody>
          <tr v-for="r in d.totals" :key="r.key" :class="r.key === 'grand' ? 'bg-doc-navy text-body font-semibold text-doc-paper' : ''">
            <td class="border border-doc-navy px-3 py-1.5">
              {{ r.label }}<span v-if="r.rate" class="ms-1">(<span class="num">{{ formatNumber(r.rate) }}%</span>)</span>
            </td>
            <td class="border border-doc-navy px-3 py-1.5 text-end">
              <template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" />
            </td>
          </tr>
        </tbody>
      </table>
    </section>

    <!-- Signatures -->
    <section class="mt-auto grid grid-cols-[1fr_1fr_1fr_auto] gap-4 pt-8 text-tiny">
      <div class="border border-doc-navy">
        <p class="border-b border-doc-navy px-3 py-1 text-center font-medium">المستلم</p>
        <div class="h-16" />
      </div>
      <div class="border border-doc-navy">
        <p class="border-b border-doc-navy px-3 py-1 text-center font-medium">المحاسب</p>
        <div class="grid h-16 place-items-center"><img v-if="d.store.signature" :src="d.store.signature" alt="" class="max-h-14 object-contain" /></div>
      </div>
      <div class="border border-doc-navy">
        <p class="border-b border-doc-navy px-3 py-1 text-center font-medium">الختم</p>
        <div class="grid h-16 place-items-center"><img v-if="d.store.stamp" :src="d.store.stamp" alt="" class="max-h-14 object-contain" /></div>
      </div>
      <QrCode v-if="d.qr" :value="d.qr" size="26mm" />
    </section>

    <footer class="mt-4 border-t border-doc-navy pt-2 text-center text-caption text-print-muted">
      <p v-if="d.footer">{{ d.footer }}</p>
      <p v-if="d.sample" class="font-medium text-doc-ink">— نموذج اختبار طباعة، ليست فاتورة حقيقية —</p>
    </footer>
  </article>
</template>
