<script setup lang="ts">
import { reactive, toRef } from 'vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { formatDigits, formatNumber } from '@/modules/core/helpers/format';
import { useInvoiceDoc } from '../../../controllers/useInvoiceDoc';
import type { PrintData } from '../../../services/invoiceService';
import QrCode from '../../QrCode.vue';

/** Plan 22 · خطاب رسمي — reads like a business letter: letterhead, salutation, a covering paragraph with the amount in words, the table, a signature. */
const props = defineProps<{ data: PrintData }>();
const d = reactive(useInvoiceDoc(toRef(props, 'data')));
</script>

<template>
  <article class="flex min-h-[296mm] w-[210mm] flex-col bg-doc-paper text-body leading-loose text-doc-ink" dir="rtl">
    <!-- Letterhead -->
    <header class="px-[18mm] pt-[14mm]">
      <div class="flex items-end justify-between gap-6">
        <div class="flex items-center gap-4">
          <img v-if="d.store.logo" :src="d.store.logo" alt="" class="size-16 object-contain" />
          <div>
            <p class="text-heading font-semibold text-doc-burgundy">{{ d.store.storeName }}</p>
            <p v-if="d.store.commercialRegister" class="text-tiny text-print-muted">{{ d.profile.commercialRegister.label }} <span class="num">{{ formatDigits(d.store.commercialRegister) }}</span></p>
          </div>
        </div>
        <div class="text-end text-tiny leading-relaxed text-print-muted">
          <p v-if="d.storeAddress">{{ d.storeAddress }}</p>
          <p v-if="d.store.phone" class="num" dir="ltr">{{ formatDigits(d.store.phone) }}</p>
          <p v-if="d.store.vatNumber">{{ d.profile.taxId.label }} <span class="num">{{ formatDigits(d.store.vatNumber) }}</span></p>
        </div>
      </div>
      <div class="mt-3 h-0.5 bg-doc-burgundy" />
      <div class="mt-0.5 h-px bg-doc-burgundy" />
    </header>

    <main class="flex flex-1 flex-col px-[18mm] pt-6">
      <div class="flex justify-between text-label">
        <p>المرجع: {{ d.title }} رقم <span class="num font-medium">{{ d.number }}</span></p>
        <p>التاريخ: <span class="num">{{ d.dateLong }}</span></p>
      </div>

      <p class="mt-6 font-medium">
        السادة / {{ d.customer?.name ?? 'العميل الكريم' }}<template v-if="d.customer"> المحترمين</template>،
      </p>
      <p v-if="d.customer?.vatNumber" class="text-tiny text-print-muted">{{ d.profile.taxId.label }}: <span class="num">{{ formatDigits(d.customer.vatNumber) }}</span></p>
      <p class="mt-3">تحية طيبة وبعد،</p>
      <p class="mt-1 text-justify">
        نرفق لكم فاتورة بالأصناف الموضحة أدناه بتاريخ <span class="num">{{ d.date }}</span>، وقد بلغ إجماليها شاملاً الضريبة
        <span class="font-semibold text-doc-burgundy"><MoneyText :value="d.inv.grandTotal" :currency="d.currency" /></span>
        ({{ d.amountInWords }})، وطريقة السداد: {{ d.paymentLabel }}.
      </p>

      <table class="mt-6 w-full text-label">
        <thead>
          <tr class="bg-doc-burgundy-soft text-doc-burgundy">
            <th class="px-3 py-1.5 text-start font-medium">م</th>
            <th class="px-3 py-1.5 text-start font-medium">البيان</th>
            <th class="px-3 py-1.5 text-center font-medium">الكمية</th>
            <th class="px-3 py-1.5 text-center font-medium">سعر الوحدة</th>
            <th class="px-3 py-1.5 text-end font-medium">القيمة</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="l in d.lines" :key="l.id" class="border-b border-print-rule">
            <td class="px-3 py-1.5"><span class="num">{{ formatNumber(l.no) }}</span></td>
            <td class="px-3 py-1.5">{{ l.name }}</td>
            <td class="px-3 py-1.5 text-center"><span class="num">{{ formatNumber(l.qty) }}</span></td>
            <td class="px-3 py-1.5 text-center"><MoneyText :value="l.price" plain /></td>
            <td class="px-3 py-1.5 text-end"><MoneyText :value="l.amount" plain /></td>
          </tr>
        </tbody>
      </table>

      <dl class="ms-auto mt-4 w-[76mm] text-label">
        <div v-for="r in d.totals" :key="r.key" class="flex justify-between py-0.5" :class="r.key === 'grand' ? 'mt-1 border-t border-doc-burgundy pt-1.5 text-body font-semibold text-doc-burgundy' : ''">
          <dt :class="r.key === 'grand' ? '' : 'text-print-muted'">{{ r.label }}<span v-if="r.rate" class="num mx-1">{{ formatNumber(r.rate) }}%</span></dt>
          <dd><template v-if="r.negative">− </template><MoneyText :value="r.value" :currency="d.currency" /></dd>
        </div>
      </dl>

      <p v-if="d.note" class="mt-4 text-label">ملاحظة: {{ d.note }}</p>
      <p class="mt-6">وتفضلوا بقبول فائق الاحترام والتقدير،</p>

      <div class="mt-4 flex items-end justify-between">
        <div class="text-center">
          <p class="font-medium">{{ d.store.storeName }}</p>
          <div class="relative mt-1 grid h-20 w-48 place-items-center">
            <img v-if="d.store.stamp" :src="d.store.stamp" alt="" class="absolute max-h-20 object-contain opacity-80" />
            <img v-if="d.store.signature" :src="d.store.signature" alt="" class="relative max-h-16 object-contain" />
          </div>
          <p class="border-t border-doc-ink pt-1 text-tiny text-print-muted">التوقيع والختم</p>
        </div>
        <QrCode v-if="d.qr" :value="d.qr" size="26mm" :border="1" />
      </div>
    </main>

    <footer class="mt-8 bg-doc-burgundy-soft px-[18mm] py-3 text-center text-tiny text-doc-burgundy">
      <p v-if="d.footer">{{ d.footer }}</p>
      <p v-if="d.sample" class="font-medium text-doc-ink">— نموذج اختبار طباعة، ليست فاتورة حقيقية —</p>
      <p v-if="!d.footer && !d.sample">{{ d.store.storeName }}</p>
    </footer>
  </article>
</template>
