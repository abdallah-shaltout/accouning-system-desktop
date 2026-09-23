<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Undo2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { invoiceOutstanding, round2 } from '../helpers/totals';
import { createRefund, getInvoice } from '../services/invoiceService';

const route = useRoute();
const router = useRouter();
const toast = useToast();
const confirm = useConfirm();
const id = String(route.params.id);

const { data, error, reload } = useAsync(() => getInvoice(id));
const qty = ref<Record<string, number>>({});
const reason = ref('المقاس غير مناسب');
const customReason = ref('');
const saving = ref(false);

const reasons = ['المقاس غير مناسب', 'عيب في المنتج', 'تغيير رأي العميل', 'اللون مختلف عن المتوقع', 'أخرى'];

watch(data, (d) => {
  if (d) qty.value = Object.fromEntries(d.lines.map((l) => [l.id, 0]));
});

const returnable = (lineId: string, sold: number) => sold - (data.value?.returnedQty[lineId] ?? 0);

/** Client-side estimate mirroring the backend math (the backend is authoritative). */
const estimate = computed(() => {
  const d = data.value;
  if (!d) return { net: 0, tax: 0, total: 0, toAccount: 0, cash: 0 };
  const net = round2(
    d.lines.reduce((a, l) => a + num0(qty.value[l.id]) * (l.price - l.discount / l.qty), 0) * (1 - d.discountRate / 100),
  );
  const tax = round2((net * d.taxRate) / 100);
  const total = round2(net + tax);
  const toAccount = Math.min(total, invoiceOutstanding(d));
  return { net, tax, total, toAccount, cash: round2(total - toAccount) };
});
const count = computed(() => Object.values(qty.value).reduce((a, n) => a + num0(n), 0));

function returnAll() {
  if (!data.value) return;
  for (const l of data.value.lines) qty.value[l.id] = returnable(l.id, l.qty);
}

async function submit() {
  const d = data.value;
  if (!d || !count.value) return;
  const ok = await confirm({
    title: `تأكيد إرجاع ${formatNumber(count.value)} قطعة؟`,
    message: 'سيُعاد المخزون وتُسجل القيود المحاسبية للمرتجع. لا يمكن التراجع.',
    confirmText: 'تأكيد الإرجاع',
  });
  if (!ok) return;
  saving.value = true;
  try {
    const refund = await createRefund({
      invoiceId: id,
      reason: reason.value === 'أخرى' ? customReason.value.trim() || undefined : reason.value,
      lines: d.lines.filter((l) => num0(qty.value[l.id]) > 0).map((l) => ({ invoiceLineId: l.id, qty: num0(qty.value[l.id]) })),
    });
    toast.success('تم تسجيل المرتجع', `${refund.number} — ${refund.cashBack > 0 ? `يُرد للعميل ${refund.cashBack.toFixed(2)}` : 'خُصم من حساب العميل'}`);
    router.push(`/invoices/${id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader :title="data ? `إرجاع من الفاتورة ${data.number}` : 'مرتجع مبيعات'" :subtitle="data?.customerName ?? 'عميل نقدي'" :back="`/invoices/${id}`" />

    <ErrorState v-if="error" :message="error" @retry="reload" />
    <div v-else class="grid items-start gap-5 xl:grid-cols-[1fr_320px]">
      <AppCard padding="none">
        <template #actions><AppButton size="sm" variant="ghost" @click="returnAll">إرجاع الكل</AppButton></template>
        <div v-if="!data" class="p-4"><SkeletonBlock :lines="5" height="h-8" /></div>
        <table v-else class="w-full text-[13px]">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2.5 text-start font-medium">الصنف</th>
              <th class="px-3 py-2.5 text-start font-medium">المباع</th>
              <th class="px-3 py-2.5 text-start font-medium">المرتجع سابقاً</th>
              <th class="px-3 py-2.5 text-start font-medium">السعر</th>
              <th class="px-4 py-2.5 text-start font-medium">كمية الإرجاع</th>
            </tr>
          </thead>
          <tbody class="bg-background">
            <tr v-for="l in data.lines" :key="l.id" class="border-b border-border last:border-0" :class="returnable(l.id, l.qty) <= 0 && 'opacity-50'">
              <td class="px-4 py-2">{{ l.name }}</td>
              <td class="px-3 py-2"><span class="num">{{ formatNumber(l.qty) }}</span></td>
              <td class="px-3 py-2"><span class="num text-text-secondary">{{ formatNumber(data.returnedQty[l.id] ?? 0) }}</span></td>
              <td class="px-3 py-2"><MoneyText :value="l.price" plain /></td>
              <td class="px-4 py-2">
                <input
                  v-model.number="qty[l.id]"
                  type="number"
                  min="0"
                  :max="returnable(l.id, l.qty)"
                  :disabled="returnable(l.id, l.qty) <= 0"
                  class="control h-8 w-20"
                  :aria-invalid="num0(qty[l.id]) > returnable(l.id, l.qty) || undefined"
                  @focus="($event.target as HTMLInputElement).select()"
                />
                <span class="ms-2 text-xs text-text-secondary">من <span class="num">{{ formatNumber(returnable(l.id, l.qty)) }}</span></span>
              </td>
            </tr>
          </tbody>
        </table>
      </AppCard>

      <div class="space-y-4">
        <AppCard title="سبب الإرجاع" padding="sm">
          <AppSelect v-model="reason" :options="reasons.map((r) => ({ value: r, label: r }))" />
          <input v-if="reason === 'أخرى'" v-model="customReason" class="control mt-2" placeholder="اكتب السبب" />
        </AppCard>
        <AppCard title="المبلغ المسترد" padding="sm">
          <dl class="space-y-1.5 text-[13px]">
            <div class="flex justify-between"><dt class="text-text-secondary">قبل الضريبة</dt><dd><MoneyText :value="estimate.net" /></dd></div>
            <div class="flex justify-between"><dt class="text-text-secondary">الضريبة</dt><dd><MoneyText :value="estimate.tax" /></dd></div>
            <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>الإجمالي</dt><dd><MoneyText :value="estimate.total" /></dd></div>
            <div v-if="estimate.toAccount" class="flex justify-between text-text-secondary"><dt>يُخصم من رصيد العميل</dt><dd><MoneyText :value="estimate.toAccount" /></dd></div>
            <div v-if="estimate.cash" class="flex justify-between text-success"><dt>يُرد للعميل</dt><dd><MoneyText :value="estimate.cash" /></dd></div>
          </dl>
          <p class="mt-3 text-[11px] leading-5 text-text-secondary">تُعاد الكميات للمخزون ويُسجل قيد: مرتجعات المبيعات والضريبة مقابل الصندوق/البنك أو حساب العميل.</p>
        </AppCard>
        <AppButton variant="primary" block :icon="Undo2" :disabled="!count" :loading="saving" @click="submit">تسجيل المرتجع</AppButton>
      </div>
    </div>
  </div>
</template>
