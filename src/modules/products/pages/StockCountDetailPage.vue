<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { CircleCheck, EyeOff, ScanBarcode, Undo2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import {
  completeStockCount,
  getStockCount,
  resumeCounting,
  submitCountForReview,
  updateStockCountLine,
} from '../services/inventoryService';
import { getProducts } from '../services/productService';
import type { Product } from '../types';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const id = String(route.params.id);

const count = useAsync(() => getStockCount(id));
const products = ref<Map<string, Product>>(new Map());
const scan = ref('');
const busy = ref(false);

onMounted(async () => {
  products.value = new Map((await getProducts({ includeInactive: true })).map((p) => [p.id, p]));
});

const c = computed(() => count.data.value);
const canWrite = computed(() => auth.can('inventory', 'write'));

function productOf(productId: string) {
  return products.value.get(productId);
}

async function onScan() {
  const code = scan.value.trim();
  scan.value = '';
  if (!code || !c.value) return;
  const product = [...products.value.values()].find((p) => p.barcode === code || p.sku.toLowerCase() === code.toLowerCase());
  if (!product) return toast.warning('لم يتم العثور على منتج', code);
  if (!c.value.lines.some((l) => l.productId === product.id)) return toast.warning('الصنف خارج نطاق هذا الجرد', product.name);
  const line = c.value.lines.find((l) => l.productId === product.id)!;
  const next = (line.countedQty ?? 0) + 1;
  line.countedQty = next; // optimistic
  try {
    await updateStockCountLine(id, product.id, 1, true);
  } catch (err) {
    toast.error(err);
    count.reload();
  }
}

async function setCounted(productId: string, qty: number) {
  if (!c.value) return;
  const line = c.value.lines.find((l) => l.productId === productId);
  if (!line) return;
  const prev = line.countedQty;
  line.countedQty = Number.isFinite(qty) ? qty : undefined; // optimistic, mirrors onScan()
  try {
    await updateStockCountLine(id, productId, qty, false);
  } catch (err) {
    toast.error(err);
    line.countedQty = prev;
  }
}

async function submitReview() {
  const missing = c.value?.lines.filter((l) => l.countedQty === undefined).length ?? 0;
  if (missing > 0) {
    const ok = await confirm({ title: `${missing} صنف لم يُعد بعد`, message: 'أكمل عدّ جميع الأصناف قبل المتابعة للمراجعة.', confirmText: 'حسناً' });
    void ok;
    return;
  }
  busy.value = true;
  try {
    await submitCountForReview(id);
    toast.success('تم إرسال الجرد للمراجعة');
    count.reload();
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}

async function backToCount() {
  busy.value = true;
  try {
    await resumeCounting(id);
    count.reload();
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}

async function apply() {
  const ok = await confirm({
    title: 'اعتماد نتيجة الجرد؟',
    message: 'سيُحدَّث المخزون بالفروقات ويُسجل قيد محاسبي (فروقات جرد المخزون 5110).',
    confirmText: 'اعتماد',
  });
  if (!ok) return;
  busy.value = true;
  try {
    const adj = await completeStockCount(id);
    toast.success('تم اعتماد الجرد', adj.number);
    router.push(`/inventory/adjustments/${adj.id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}

const variance = computed(() => {
  if (!c.value) return { gains: 0, losses: 0, count: 0 };
  let gains = 0;
  let losses = 0;
  let diffCount = 0;
  for (const l of c.value.lines) {
    const diff = (l.countedQty ?? 0) - l.systemQty;
    if (Math.abs(diff) > 0.0001) {
      diffCount++;
      const value = diff * l.unitCost;
      if (value > 0) gains += value;
      else losses += -value;
    }
  }
  return { gains, losses, count: diffCount };
});
</script>

<template>
  <div>
    <ErrorState v-if="count.error.value" :message="count.error.value" @retry="count.reload" />
    <template v-else>
      <PageHeader :title="c ? `جرد ${c.number}` : '…'" back="/inventory/counts">
        <template v-if="c" #badge>
          <StatusBadge v-if="c.blind" tone="primary" label="أعمى" :dot="false" />
          <StatusBadge
            :tone="c.status === 'COMPLETED' ? 'success' : c.status === 'REVIEW' ? 'neutral' : 'warning'"
            :label="c.status === 'COMPLETED' ? 'مكتمل' : c.status === 'REVIEW' ? 'قيد المراجعة' : 'جارٍ العد'"
          />
        </template>
        <template v-if="c && canWrite && c.status !== 'COMPLETED'" #actions>
          <template v-if="c.status === 'OPEN'">
            <AppButton variant="primary" :icon="CircleCheck" :loading="busy" @click="submitReview">إرسال للمراجعة</AppButton>
          </template>
          <template v-else-if="c.status === 'REVIEW'">
            <AppButton :icon="Undo2" :loading="busy" @click="backToCount">العودة للعدّ</AppButton>
            <AppButton variant="primary" :icon="CircleCheck" :loading="busy" @click="apply">اعتماد النتيجة</AppButton>
          </template>
        </template>
      </PageHeader>

      <AppCard v-if="!c"><SkeletonBlock :lines="6" /></AppCard>

      <template v-else>
        <AppCard v-if="c.status === 'OPEN'" padding="sm" class="mb-4">
          <div class="flex items-center gap-3">
            <div class="relative flex-1">
              <ScanBarcode class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-text-secondary" />
              <input v-model="scan" class="control ps-8" placeholder="امسح الباركود لزيادة الكمية المعدودة بمقدار 1" @keydown.enter.prevent="onScan" />
            </div>
            <span v-if="c.blind" class="inline-flex items-center gap-1.5 text-xs text-text-secondary"><EyeOff class="size-3.5" /> عد أعمى — رصيد النظام مخفي</span>
          </div>
        </AppCard>

        <AppCard v-if="c.status === 'REVIEW' || c.status === 'COMPLETED'" padding="sm" class="mb-4">
          <dl class="flex flex-wrap gap-6 text-body">
            <div><dt class="text-xs text-text-secondary">أصناف بها فروقات</dt><dd class="num font-medium">{{ formatNumber(variance.count) }} من {{ formatNumber(c.lines.length) }}</dd></div>
            <div><dt class="text-xs text-text-secondary">زيادات</dt><dd><MoneyText :value="variance.gains" /></dd></div>
            <div><dt class="text-xs text-text-secondary">عجز</dt><dd><MoneyText :value="variance.losses" /></dd></div>
          </dl>
        </AppCard>

        <AppCard padding="none">
          <div class="max-h-[65vh] overflow-y-auto">
            <table class="w-full text-body">
              <thead class="sticky top-0 z-[1] bg-surface text-xs text-text-secondary">
                <tr class="border-b border-border">
                  <th class="px-4 py-2 text-start font-medium">الصنف</th>
                  <th v-if="!c.blind || c.status !== 'OPEN'" class="px-2 py-2 text-start font-medium">رصيد النظام</th>
                  <th class="px-2 py-2 text-start font-medium">المعدود</th>
                  <th v-if="c.status !== 'OPEN'" class="px-2 py-2 text-start font-medium">الفرق</th>
                  <th v-if="c.status !== 'OPEN'" class="px-4 py-2 text-start font-medium">القيمة</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="line in c.lines" :key="line.productId" class="border-b border-border last:border-0" :class="c.status !== 'OPEN' && (line.countedQty ?? 0) - line.systemQty !== 0 && 'bg-warning/5'">
                  <td class="px-4 py-1.5">
                    {{ productOf(line.productId)?.name ?? '…' }}
                    <span class="num block text-tiny text-text-secondary">{{ productOf(line.productId)?.sku }}</span>
                  </td>
                  <td v-if="!c.blind || c.status !== 'OPEN'" class="px-2 py-1.5"><span class="num text-text-secondary">{{ formatNumber(line.systemQty) }}</span></td>
                  <td class="px-2 py-1.5">
                    <input
                      v-if="c.status === 'OPEN' && canWrite"
                      :value="line.countedQty"
                      type="number"
                      min="0"
                      class="control h-8 w-24"
                      @change="setCounted(line.productId, Number(($event.target as HTMLInputElement).value))"
                    />
                    <span v-else class="num">{{ formatNumber(line.countedQty) }}</span>
                  </td>
                  <td v-if="c.status !== 'OPEN'" class="px-2 py-1.5">
                    <span class="num font-medium" :class="(line.countedQty ?? 0) - line.systemQty > 0 ? 'text-success' : (line.countedQty ?? 0) - line.systemQty < 0 ? 'text-danger' : 'text-text-secondary'">
                      {{ (line.countedQty ?? 0) - line.systemQty > 0 ? '+' : '' }}{{ formatNumber((line.countedQty ?? 0) - line.systemQty) }}
                    </span>
                  </td>
                  <td v-if="c.status !== 'OPEN'" class="px-4 py-1.5">
                    <MoneyText :value="((line.countedQty ?? 0) - line.systemQty) * line.unitCost" signed dash-zero />
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </AppCard>
      </template>
    </template>
  </div>
</template>
