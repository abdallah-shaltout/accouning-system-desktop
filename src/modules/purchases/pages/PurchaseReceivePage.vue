<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { AlertTriangle, PackageCheck, Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, formatNumber, todayKey } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { roleCanSeePurchasePrices } from '@/modules/users/helpers/permissions';
import { getPurchaseOrder, receivePurchaseOrder } from '../services/purchaseService';
import type { ReceiveLineInput } from '../types';

const route = useRoute();
const router = useRouter();
const toast = useToast();
const confirm = useConfirm();
const auth = useAuthStore();
const id = String(route.params.id);

const { data, error, reload } = useAsync(() => getPurchaseOrder(id));
const date = ref(todayKey());
const receivedQty = ref<Record<string, number>>({});
const batchNo = ref<Record<string, string>>({});
const expiryDate = ref<Record<string, string>>({});
const createBackorder = ref(true);
const saving = ref(false);

const canSeePrices = computed(() => roleCanSeePurchasePrices(auth.role));

watch(
  data,
  (d) => {
    if (!d) return;
    if (d.status !== 'DRAFT' && d.status !== 'ORDERED') router.replace(`/purchases/${id}`);
    for (const l of d.lines) {
      const remaining = Math.max(0, l.qty * (l.unitFactor ?? 1) - (l.receivedQty ?? 0));
      if (receivedQty.value[l.productId] === undefined) receivedQty.value[l.productId] = remaining;
    }
  },
  { immediate: true },
);

function remainingFor(productId: string): number {
  const l = data.value?.lines.find((x) => x.productId === productId);
  if (!l) return 0;
  return Math.max(0, l.qty * (l.unitFactor ?? 1) - (l.receivedQty ?? 0));
}

const hasShortDelivery = computed(() => (data.value?.lines ?? []).some((l) => num0(receivedQty.value[l.productId]) < remainingFor(l.productId)));
const totalToReceive = computed(() => Object.values(receivedQty.value).reduce((a, n) => a + num0(n), 0));

async function confirmReceipt() {
  if (!data.value) return;
  const short = hasShortDelivery.value;
  const ok = await confirm({
    title: 'تأكيد استلام البضاعة؟',
    message: short ? 'الكمية المستلمة أقل من المطلوب — سيُنشأ أمر متبقٍ تلقائياً للفرق إن اخترت ذلك.' : 'سيتم تحديث المخزون وترحيل القيد المحاسبي بأسعار الأمر.',
    confirmText: 'تأكيد الاستلام',
  });
  if (!ok) return;
  saving.value = true;
  try {
    const lines: ReceiveLineInput[] = data.value.lines
      .filter((l) => num0(receivedQty.value[l.productId]) > 0)
      .map((l) => {
        const product = data.value!.products[l.productId];
        const input: ReceiveLineInput = { productId: l.productId, receivedQty: num0(receivedQty.value[l.productId]) };
        if (product?.trackBatches && batchNo.value[l.productId]?.trim()) {
          input.batches = [{ batchNo: batchNo.value[l.productId].trim(), expiryDate: expiryDate.value[l.productId] || undefined, qty: num0(receivedQty.value[l.productId]) }];
        }
        return input;
      });
    if (!lines.length) {
      toast.error('أدخل كمية استلام واحدة على الأقل');
      return;
    }
    await receivePurchaseOrder(id, { date: dateKeyToIso(date.value), lines, createBackorder: createBackorder.value && short });
    toast.success('تم تأكيد الاستلام');
    router.push(`/purchases/${id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

function printLabels() {
  // TODO(phase 11b): wire to the label builder (docs/v2/07-products-and-inventory.md §6 "طباعة
  // ملصقات للكميات المستلمة"). Phase 8 only wires the button; the builder itself is Phase 11b's job.
  toast.info('طباعة الملصقات ستتوفر مع منشئ الملصقات (المرحلة 11ب)');
}
</script>

<template>
  <div>
    <PageHeader :title="data ? `استلام أمر شراء ${data.number}` : 'استلام أمر شراء'" :subtitle="data?.supplierName" :back="`/purchases/${id}`" />
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <div v-else class="grid items-start gap-5 xl:grid-cols-[1fr_320px]">
      <div class="space-y-4">
        <div v-if="!canSeePrices" class="flex items-start gap-2 rounded-lg border border-border bg-surface px-3 py-2.5 text-xs text-text-secondary">
          <AlertTriangle class="mt-0.5 size-4 shrink-0" />
          <span>الأسعار مخفية عن دورك الحالي — أدخل الكميات المستلمة فقط، وسيتم الترحيل بأسعار الأمر تلقائياً.</span>
        </div>
        <AppCard padding="none">
          <div v-if="!data" class="p-4"><SkeletonBlock :lines="5" height="h-10" /></div>
          <table v-else class="w-full text-body">
            <thead class="bg-surface text-xs text-text-secondary">
              <tr class="border-b border-border">
                <th class="px-4 py-2.5 text-start font-medium">الصنف</th>
                <th class="px-3 py-2.5 text-start font-medium">المطلوب</th>
                <th class="px-3 py-2.5 text-start font-medium">المستلم سابقاً</th>
                <th v-if="canSeePrices" class="px-3 py-2.5 text-start font-medium">سعر التكلفة</th>
                <th class="w-28 px-3 py-2.5 text-start font-medium">الكمية المستلمة</th>
                <th class="px-3 py-2.5 text-start font-medium">التشغيلة</th>
                <th class="px-3 py-2.5 text-start font-medium">الصلاحية</th>
              </tr>
            </thead>
            <tbody class="bg-background">
              <tr v-for="l in data.lines" :key="l.productId" class="border-b border-border last:border-0">
                <td class="px-4 py-2">
                  {{ data.products[l.productId]?.name }}
                  <span class="num block text-tiny text-text-secondary">{{ data.products[l.productId]?.sku }}</span>
                </td>
                <td class="px-3 py-2"><span class="num">{{ formatNumber(l.qty * (l.unitFactor ?? 1)) }}</span></td>
                <td class="px-3 py-2"><span class="num text-text-secondary">{{ formatNumber(l.receivedQty ?? 0) }}</span></td>
                <td v-if="canSeePrices" class="px-3 py-2"><MoneyText :value="l.costPrice" plain /></td>
                <td class="px-3 py-2">
                  <input v-model.number="receivedQty[l.productId]" type="number" min="0" :max="remainingFor(l.productId)" step="any" class="control num h-8 w-24" />
                </td>
                <td class="px-3 py-2">
                  <input v-if="data.products[l.productId]?.trackBatches" v-model="batchNo[l.productId]" type="text" placeholder="رقم التشغيلة" class="control h-8 w-28" />
                  <span v-else class="text-tiny text-text-secondary">—</span>
                </td>
                <td class="px-3 py-2">
                  <input v-if="data.products[l.productId]?.trackBatches" v-model="expiryDate[l.productId]" type="date" class="control h-8 w-36" />
                  <span v-else class="text-tiny text-text-secondary">—</span>
                </td>
              </tr>
            </tbody>
          </table>
        </AppCard>
        <AppButton :icon="Printer" @click="printLabels">طباعة ملصقات للكميات المستلمة</AppButton>
      </div>

      <div class="space-y-4">
        <AppCard title="تأكيد الاستلام" padding="sm">
          <div class="space-y-3">
            <AppInput v-model="date" type="date" label="تاريخ الاستلام" />
            <p class="text-xs text-text-secondary">إجمالي الكمية المستلمة: <span class="num font-medium text-text-primary">{{ formatNumber(totalToReceive) }}</span></p>
            <AppSwitch v-if="hasShortDelivery" v-model="createBackorder" label="إنشاء أمر متبقٍ للفرق" description="أمر شراء مسودة جديد بالكمية غير المستلمة" />
            <p v-if="hasShortDelivery" class="rounded-lg border border-warning/40 bg-warning/10 px-2.5 py-2 text-xs text-warning">
              تسليم جزئي — سيتم استلام هذا الأمر مرة واحدة فقط بالكميات المدخلة.
            </p>
            <AppButton variant="primary" block :icon="PackageCheck" :loading="saving" @click="confirmReceipt">تأكيد الاستلام</AppButton>
          </div>
        </AppCard>
      </div>
    </div>
  </div>
</template>
