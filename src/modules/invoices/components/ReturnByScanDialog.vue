<script setup lang="ts">
/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §1 "Return (F7): scan the receipt QR or type its number
 * → pick the lines → refund method. Returning without a receipt needs sales.refundWithoutReceipt.").
 * The ZATCA QR encodes seller/VAT/timestamp/totals (see helpers/zatcaQr.ts), not the invoice number,
 * so "scan the receipt QR" in this mock UI means scanning/typing the printed invoice number barcode
 * — the same input a barcode scanner would feed a text field either way.
 */
import { computed, ref, watch } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { Search } from '@lucide/vue';
import { getInvoice, getInvoices, createRefund, type InvoiceDetail } from '../services/invoiceService';
import type { RefundMethod } from '../types';

const props = defineProps<{ allowWithoutReceipt: boolean }>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ done: [] }>();
const toast = useToast();

const code = ref('');
const searching = ref(false);
const invoice = ref<InvoiceDetail | null>(null);
const notFound = ref(false);
const qty = ref<Record<string, number>>({});
const restock = ref<Record<string, boolean>>({});
const reason = ref('رغبة العميل');
const refundMethod = ref<RefundMethod>('cash');
const saving = ref(false);

const REASONS = ['عيب مصنعي', 'مقاس غير مناسب', 'رغبة العميل', 'خطأ في الفاتورة', 'أخرى'];

watch(open, (o) => {
  if (!o) {
    code.value = '';
    invoice.value = null;
    notFound.value = false;
  }
});

async function search() {
  const value = code.value.trim();
  if (!value) return;
  searching.value = true;
  notFound.value = false;
  try {
    const matches = await getInvoices({ search: value });
    const exact = matches.find((m) => m.number.toLowerCase() === value.toLowerCase()) ?? matches[0];
    if (!exact) {
      notFound.value = true;
      return;
    }
    invoice.value = await getInvoice(exact.id);
    qty.value = Object.fromEntries(invoice.value.lines.map((l) => [l.id, 0]));
    restock.value = Object.fromEntries(invoice.value.lines.map((l) => [l.id, true]));
  } catch (err) {
    toast.error(err);
  } finally {
    searching.value = false;
  }
}

function returnable(lineId: string, sold: number) {
  return sold - (invoice.value?.returnedQty[lineId] ?? 0);
}

const count = computed(() => Object.values(qty.value).reduce((a, n) => a + num0(n), 0));

async function submit() {
  if (!invoice.value || !count.value) return;
  saving.value = true;
  try {
    const refund = await createRefund({
      invoiceId: invoice.value.id,
      reason: reason.value === 'أخرى' ? undefined : reason.value,
      refundMethod: refundMethod.value,
      lines: invoice.value.lines.filter((l) => num0(qty.value[l.id]) > 0).map((l) => ({ invoiceLineId: l.id, qty: num0(qty.value[l.id]), restock: restock.value[l.id] })),
    });
    toast.success('تم تسجيل المرتجع', refund.number);
    open.value = false;
    emit('done');
  } catch (err) {
    toast.error(err, 'تعذر تسجيل المرتجع');
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <AppModal v-model:open="open" title="إرجاع بمسح الإيصال" size="lg">
    <div class="space-y-4">
      <div class="flex gap-2">
        <input v-model="code" class="control h-11 flex-1" placeholder="امسح رقم الفاتورة أو اكتبه" autofocus @keydown.enter.prevent="search" />
        <AppButton :icon="Search" :loading="searching" @click="search">بحث</AppButton>
      </div>
      <p v-if="notFound" class="text-body text-danger">لا توجد فاتورة بهذا الرقم.</p>
      <p v-if="notFound && !allowWithoutReceipt" class="text-xs text-text-secondary">الإرجاع بدون إيصال غير مسموح به لدورك الحالي.</p>

      <div v-if="invoice" class="space-y-4">
        <div class="rounded-lg border border-border bg-surface p-3 text-body">
          <p class="font-medium">{{ invoice.number }} — {{ invoice.customerName ?? 'عميل نقدي' }}</p>
          <p class="text-xs text-text-secondary"><MoneyText :value="invoice.grandTotal" plain /></p>
        </div>
        <table class="w-full text-body">
          <thead class="text-xs text-text-secondary">
            <tr>
              <th class="px-2 py-1.5 text-start">الصنف</th>
              <th class="px-2 py-1.5 text-start">المتاح للإرجاع</th>
              <th class="px-2 py-1.5 text-start">الكمية</th>
              <th class="px-2 py-1.5 text-start">إعادة للمخزون</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="l in invoice.lines" :key="l.id" :class="returnable(l.id, l.qty) <= 0 && 'opacity-50'">
              <td class="px-2 py-1.5">{{ l.name }}</td>
              <td class="px-2 py-1.5"><span class="num">{{ formatNumber(returnable(l.id, l.qty)) }}</span></td>
              <td class="px-2 py-1.5">
                <input v-model.number="qty[l.id]" type="number" min="0" :max="returnable(l.id, l.qty)" :disabled="returnable(l.id, l.qty) <= 0" class="control h-8 w-20" />
              </td>
              <td class="px-2 py-1.5">
                <input v-model="restock[l.id]" type="checkbox" class="size-4" :disabled="l.isFreeText" />
              </td>
            </tr>
          </tbody>
        </table>

        <div class="grid grid-cols-2 gap-3">
          <AppSelect v-model="reason" label="سبب الإرجاع" :options="REASONS.map((r) => ({ value: r, label: r }))" />
          <AppSelect
            v-model="refundMethod"
            label="طريقة الاسترداد"
            :options="[
              { value: 'cash', label: 'نقداً' },
              { value: 'card', label: 'بطاقة' },
              { value: 'bank_transfer', label: 'تحويل بنكي' },
              { value: 'customer_credit', label: 'رصيد للعميل', disabled: !invoice.customerId },
            ]"
          />
        </div>
      </div>
      <EmptyState v-else-if="!notFound" :icon="Search" title="امسح أو اكتب رقم الفاتورة" description="سيظهر تفاصيل البيع لاختيار الأصناف المرتجعة" />
    </div>
    <template #footer>
      <AppButton @click="open = false">إلغاء</AppButton>
      <AppButton v-if="invoice" variant="primary" :disabled="!count" :loading="saving" @click="submit">تسجيل المرتجع</AppButton>
    </template>
  </AppModal>
</template>
