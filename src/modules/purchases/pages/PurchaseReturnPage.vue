<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Undo2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { round2 } from '@/modules/invoices/helpers/totals';
import { createPurchaseReturn, getPurchaseOrder } from '../services/purchaseService';

const route = useRoute();
const router = useRouter();
const toast = useToast();
const confirm = useConfirm();
const id = String(route.params.id);

const { data, error, reload } = useAsync(() => getPurchaseOrder(id));
const qty = ref<Record<string, number>>({});
const reason = ref('عيوب تصنيع');
const saving = ref(false);

watch(data, (d) => {
  if (!d) return;
  if (d.status !== 'CONFIRMED') router.replace(`/purchases/${id}`);
  qty.value = Object.fromEntries(d.lines.map((l) => [l.productId, 0]));
});

/** Can't return more than was bought (minus earlier returns), nor more than is still in stock. */
function maxReturn(productId: string, bought: number) {
  const d = data.value!;
  const left = bought - (d.returnedQty[productId] ?? 0);
  const info = d.products[productId];
  return info?.type === 'product' ? Math.min(left, info.stockQty) : left;
}

const totals = computed(() => {
  const d = data.value;
  if (!d) return { sub: 0, tax: 0, total: 0 };
  const sub = round2(d.lines.reduce((a, l) => a + num0(qty.value[l.productId]) * l.costPrice, 0));
  const tax = round2((sub * d.taxRate) / 100);
  return { sub, tax, total: round2(sub + tax) };
});
const count = computed(() => Object.values(qty.value).reduce((a, n) => a + num0(n), 0));
const invalid = computed(() => data.value?.lines.some((l) => num0(qty.value[l.productId]) > maxReturn(l.productId, l.qty)) ?? false);

async function submit() {
  if (!data.value || !count.value || invalid.value) return;
  const ok = await confirm({ title: 'تأكيد المرتجع للمورد؟', message: 'ستُخصم الكميات من المخزون ويُخفض رصيد المورد.', confirmText: 'تأكيد' });
  if (!ok) return;
  saving.value = true;
  try {
    const ret = await createPurchaseReturn({
      purchaseOrderId: id,
      reason: reason.value.trim() || undefined,
      lines: data.value.lines.filter((l) => num0(qty.value[l.productId]) > 0).map((l) => ({ productId: l.productId, qty: num0(qty.value[l.productId]) })),
    });
    toast.success('تم تسجيل المرتجع', ret.number);
    router.push(`/purchases/${id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader :title="data ? `مرتجع مشتريات — ${data.number}` : 'مرتجع مشتريات'" :subtitle="data?.supplierName" :back="`/purchases/${id}`" />
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <div v-else class="grid items-start gap-5 xl:grid-cols-[1fr_320px]">
      <AppCard padding="none">
        <div v-if="!data" class="p-4"><SkeletonBlock :lines="5" height="h-8" /></div>
        <table v-else class="w-full text-[13px]">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2.5 text-start font-medium">الصنف</th>
              <th class="px-3 py-2.5 text-start font-medium">المشترى</th>
              <th class="px-3 py-2.5 text-start font-medium">المتوفر حالياً</th>
              <th class="px-3 py-2.5 text-start font-medium">التكلفة</th>
              <th class="px-4 py-2.5 text-start font-medium">كمية الإرجاع</th>
            </tr>
          </thead>
          <tbody class="bg-background">
            <tr v-for="l in data.lines" :key="l.productId" class="border-b border-border last:border-0" :class="maxReturn(l.productId, l.qty) <= 0 && 'opacity-50'">
              <td class="px-4 py-2">{{ data.products[l.productId]?.name }}</td>
              <td class="px-3 py-2"><span class="num">{{ formatNumber(l.qty) }}</span></td>
              <td class="px-3 py-2"><span class="num text-text-secondary">{{ formatNumber(data.products[l.productId]?.stockQty) }}</span></td>
              <td class="px-3 py-2"><MoneyText :value="l.costPrice" plain /></td>
              <td class="px-4 py-2">
                <input
                  v-model.number="qty[l.productId]"
                  type="number"
                  min="0"
                  :max="maxReturn(l.productId, l.qty)"
                  :disabled="maxReturn(l.productId, l.qty) <= 0"
                  class="control h-8 w-20"
                  :aria-invalid="num0(qty[l.productId]) > maxReturn(l.productId, l.qty) || undefined"
                />
                <span class="ms-2 text-xs text-text-secondary">حد <span class="num">{{ formatNumber(maxReturn(l.productId, l.qty)) }}</span></span>
              </td>
            </tr>
          </tbody>
        </table>
      </AppCard>
      <div class="space-y-4">
        <AppCard padding="sm">
          <AppInput v-model="reason" label="سبب الإرجاع" />
          <dl class="mt-4 space-y-1.5 text-[13px]">
            <div class="flex justify-between"><dt class="text-text-secondary">قبل الضريبة</dt><dd><MoneyText :value="totals.sub" /></dd></div>
            <div class="flex justify-between"><dt class="text-text-secondary">الضريبة</dt><dd><MoneyText :value="totals.tax" /></dd></div>
            <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>قيمة المرتجع</dt><dd><MoneyText :value="totals.total" /></dd></div>
          </dl>
          <p class="mt-3 text-[11px] leading-5 text-text-secondary">يُخفض رصيد المورد بقيمة المرتجع؛ وإن كان الأمر مسدداً بالكامل يُسجل الفرق كمبلغ مسترد نقداً.</p>
        </AppCard>
        <p v-if="invalid" class="text-xs text-danger">كمية الإرجاع تتجاوز الحد المسموح لأحد الأصناف</p>
        <AppButton variant="primary" block :icon="Undo2" :disabled="!count || invalid" :loading="saving" @click="submit">تسجيل المرتجع</AppButton>
      </div>
    </div>
  </div>
</template>
