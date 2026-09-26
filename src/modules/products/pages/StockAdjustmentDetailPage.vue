<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { BookOpen, CircleCheck, Trash } from '@lucide/vue';
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
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { ADJUSTMENT_TYPE } from '@/modules/core/helpers/labels';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { adjustmentValue, completeAdjustment, deleteDraftAdjustment, getStockAdjustment } from '../services/inventoryService';
import { getProducts } from '../services/productService';
import type { Product } from '../types';

const route = useRoute('adjustment');
const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const id = String(route.params.id);

const adj = useAsync(() => getStockAdjustment(id));
const products = ref<Map<string, Product>>(new Map());
onMounted(async () => {
  products.value = new Map((await getProducts({ includeInactive: true })).map((p) => [p.id, p]));
});

const a = computed(() => adj.data.value);
const busy = ref(false);
const isDraft = computed(() => a.value?.status === 'DRAFT');

async function complete() {
  const ok = await confirm({
    title: 'اعتماد التسوية؟',
    message: a.value?.type === 'STOCKTAKE' ? 'سيُعاد احتساب الفروقات مقابل الرصيد الحالي، ثم يُحدّث المخزون ويُسجل القيد.' : 'سيُحدّث المخزون ويُسجل القيد المحاسبي.',
    confirmText: 'اعتماد',
  });
  if (!ok) return;
  busy.value = true;
  try {
    await completeAdjustment(id);
    toast.success('تم اعتماد التسوية', a.value?.number);
    adj.reload();
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}

async function remove() {
  const ok = await confirm({ title: 'حذف المسودة؟', confirmText: 'حذف', danger: true });
  if (!ok) return;
  try {
    await deleteDraftAdjustment(id);
    toast.success('تم حذف المسودة');
    router.push({ name: 'adjustments' });
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <ErrorState v-if="adj.error.value" :message="adj.error.value" @retry="adj.reload" />
    <template v-else>
      <PageHeader :title="a ? `${ADJUSTMENT_TYPE[a.type].label} ${a.number}` : '…'" :back="{ name: 'adjustments' }">
        <template v-if="a" #badge>
          <StatusBadge :tone="a.status === 'COMPLETED' ? 'success' : 'neutral'" :label="a.status === 'COMPLETED' ? 'معتمدة' : 'مسودة'" />
        </template>
        <template v-if="a" #subtitle>
          <span class="num">{{ formatDateTime(a.date) }}</span><template v-if="a.note"> · {{ a.note }}</template>
        </template>
        <template #actions>
          <AppButton v-if="a?.journalEntryId && auth.can('accounting')" :icon="BookOpen" :to="{ name: 'journal-entry', params: { id: a.journalEntryId } }">القيد المحاسبي</AppButton>
          <template v-if="isDraft && auth.can('inventory', 'write')">
            <AppButton variant="danger" :icon="Trash" @click="remove">حذف المسودة</AppButton>
            <AppButton variant="primary" :icon="CircleCheck" :loading="busy" @click="complete">اعتماد</AppButton>
          </template>
        </template>
      </PageHeader>

      <AppCard padding="none">
        <div v-if="!a" class="p-4"><SkeletonBlock :lines="6" /></div>
        <table v-else class="w-full text-body">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2.5 text-start font-medium">الصنف</th>
              <th class="px-3 py-2.5 text-start font-medium">رصيد النظام</th>
              <th v-if="a.type === 'STOCKTAKE'" class="px-3 py-2.5 text-start font-medium">المعدود</th>
              <th class="px-3 py-2.5 text-start font-medium">التغيير</th>
              <th class="px-3 py-2.5 text-start font-medium">تكلفة الوحدة</th>
              <th class="px-4 py-2.5 text-start font-medium">القيمة</th>
            </tr>
          </thead>
          <tbody class="bg-background">
            <tr v-for="l in a.lines" :key="l.productId" class="border-b border-border last:border-0">
              <td class="px-4 py-2.5">
                <RouterLink :to="{ name: 'product', params: { id: l.productId } }" class="hover:text-primary">{{ products.get(l.productId)?.name ?? '…' }}</RouterLink>
                <span class="num block text-tiny text-text-secondary">{{ products.get(l.productId)?.sku }}</span>
              </td>
              <td class="px-3 py-2.5"><span class="num text-text-secondary">{{ formatNumber(l.systemQty) }}</span></td>
              <td v-if="a.type === 'STOCKTAKE'" class="px-3 py-2.5"><span class="num">{{ formatNumber(l.countedQty) }}</span></td>
              <td class="px-3 py-2.5">
                <span class="num font-medium" :class="l.qtyChange > 0 ? 'text-success' : l.qtyChange < 0 ? 'text-danger' : 'text-text-secondary'">
                  {{ l.qtyChange > 0 ? '+' : '' }}{{ formatNumber(l.qtyChange) }}
                </span>
              </td>
              <td class="px-3 py-2.5"><MoneyText :value="l.unitCost" plain class="text-text-secondary" /></td>
              <td class="px-4 py-2.5"><MoneyText :value="l.qtyChange * (l.unitCost ?? 0)" signed dash-zero /></td>
            </tr>
          </tbody>
          <tfoot class="border-t border-border bg-surface font-medium">
            <tr>
              <td class="px-4 py-2.5" :colspan="a.type === 'STOCKTAKE' ? 5 : 4">صافي الأثر على قيمة المخزون</td>
              <td class="px-4 py-2.5"><MoneyText :value="adjustmentValue(a)" signed /></td>
            </tr>
          </tfoot>
        </table>
      </AppCard>
      <p v-if="isDraft" class="mt-3 text-xs text-text-secondary">المسودة لا تؤثر على المخزون أو الحسابات حتى يتم اعتمادها.</p>
    </template>
  </div>
</template>
