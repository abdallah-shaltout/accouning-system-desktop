<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { AlertTriangle, PackageX, Undo2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDate, formatNumber } from '@/modules/core/helpers/format';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { type ExpiryBucket, type ExpiryRow, getExpiryReport, returnBatchesToSupplier, writeOffExpiredBatches } from '../services/inventoryService';
// v2 phase 8: completes the phase-6 stub into a real posted debit note (docs/v2/09-purchases-
// payments-expenses.md "Debit notes v2" — "return expiring batch" shortcut).
import { postDebitNoteDraft } from '@/modules/purchases/services/purchaseService';
import type { RefundMethod } from '@/modules/purchases/types';

const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();

const rows = ref<ExpiryRow[]>([]);
const loading = ref(true);
const bucket = ref<ExpiryBucket | 'all'>('all');
const selected = ref<Set<string>>(new Set());
const returnOpen = ref(false);
const busy = ref(false);
const refundMethod = ref<RefundMethod>('credit');
const refundOptions: { value: RefundMethod; label: string }[] = [
  { value: 'credit', label: 'خصم من رصيد المورد (آجل)' },
  { value: 'cash', label: 'استرداد نقدي' },
  { value: 'bank_transfer', label: 'استرداد بنكي' },
];

async function load() {
  loading.value = true;
  try {
    rows.value = await getExpiryReport();
  } finally {
    loading.value = false;
  }
}
onMounted(load);

const BUCKET_LABEL: Record<ExpiryBucket, { label: string; tone: 'danger' | 'warning' | 'neutral' }> = {
  expired: { label: 'منتهي الصلاحية', tone: 'danger' },
  within30: { label: '≤ 30 يوماً', tone: 'danger' },
  within60: { label: '≤ 60 يوماً', tone: 'warning' },
  within90: { label: '≤ 90 يوماً', tone: 'warning' },
  ok: { label: '—', tone: 'neutral' },
};

const filtered = computed(() => (bucket.value === 'all' ? rows.value : rows.value.filter((r) => r.bucket === bucket.value)));

const bySupplier = computed(() => {
  const groups = new Map<string, ExpiryRow[]>();
  for (const r of filtered.value) {
    const key = r.supplierName ?? 'بدون مورد';
    groups.set(key, [...(groups.get(key) ?? []), r]);
  }
  return [...groups.entries()].sort((a, b) => a[0].localeCompare(b[0], 'ar'));
});

const counts = computed(() => ({
  all: rows.value.length,
  expired: rows.value.filter((r) => r.bucket === 'expired').length,
  within30: rows.value.filter((r) => r.bucket === 'within30').length,
  within60: rows.value.filter((r) => r.bucket === 'within60').length,
  within90: rows.value.filter((r) => r.bucket === 'within90').length,
}));

const bucketOptions = computed(() => [
  { value: 'all' as const, label: 'الكل', count: counts.value.all },
  { value: 'expired' as const, label: 'منتهي', count: counts.value.expired },
  { value: 'within30' as const, label: '≤30', count: counts.value.within30 },
  { value: 'within60' as const, label: '≤60', count: counts.value.within60 },
  { value: 'within90' as const, label: '≤90', count: counts.value.within90 },
]);

function toggle(id: string) {
  const next = new Set(selected.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  selected.value = next;
}

const selectedRows = computed(() => rows.value.filter((r) => selected.value.has(r.id)));
const canAct = computed(() => auth.can('inventory', 'write') && selectedRows.value.length > 0);

async function writeOff() {
  const ok = await confirm({
    title: `إتلاف ${selectedRows.value.length} تشغيلة؟`,
    message: 'سيُسجل إتلاف بضاعة منتهية الصلاحية (حساب البضاعة التالفة ومنتهية الصلاحية) وتُحدَّث الكميات.',
    confirmText: 'إتلاف',
    danger: true,
  });
  if (!ok) return;
  busy.value = true;
  try {
    await writeOffExpiredBatches(selectedRows.value.map((r) => r.id));
    toast.success('تم تسجيل الإتلاف');
    selected.value = new Set();
    await load();
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}

// return-to-supplier: only enabled when every selected batch shares one supplier.
const returnSupplierName = computed(() => {
  const names = new Set(selectedRows.value.map((r) => r.supplierName ?? '—'));
  return names.size === 1 ? [...names][0] : undefined;
});

/**
 * v2 phase 8: the shortcut now goes all the way to a posted debit note — draft (phase 6's stub,
 * kept as the intermediate record) then immediately posted against the batches' purchase order
 * (stock out + AP reduced), instead of leaving a DRAFT that needed a separate purchases-screen step.
 */
async function confirmReturn() {
  const first = selectedRows.value[0];
  if (!first?.supplierId) {
    toast.warning('لا يوجد مورد مرتبط بهذه التشغيلات — لا يمكن إنشاء إرجاع');
    return;
  }
  busy.value = true;
  try {
    const draft = await returnBatchesToSupplier(
      first.supplierId,
      selectedRows.value.map((r) => ({ productId: r.productId, batchId: r.id, qty: r.qty, unitCost: r.unitCost })),
      'بضاعة منتهية/قاربت على انتهاء الصلاحية — من تقرير الصلاحية',
    );
    const debitNote = await postDebitNoteDraft(draft.id, refundMethod.value);
    toast.success('تم تسجيل مرتجع المشتريات', debitNote.number);
    returnOpen.value = false;
    selected.value = new Set();
    await load();
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="تقرير الصلاحية" subtitle="التشغيلات المنتهية أو القريبة من الانتهاء، مجمّعة حسب المورد" />

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <SegmentedControl v-model="bucket" :options="bucketOptions" />
      <div v-if="auth.can('inventory', 'write')" class="flex items-center gap-2">
        <span v-if="selectedRows.length" class="text-xs text-text-secondary">{{ formatNumber(selectedRows.length) }} محددة</span>
        <AppButton size="sm" :icon="Undo2" :disabled="!canAct" @click="returnOpen = true">إرجاع للمورد</AppButton>
        <AppButton size="sm" variant="danger" :icon="PackageX" :disabled="!canAct" :loading="busy" @click="writeOff">إتلاف</AppButton>
      </div>
    </div>

    <SkeletonBlock v-if="loading" :lines="6" height="h-9" />
    <EmptyState v-else-if="!filtered.length" :icon="AlertTriangle" title="لا توجد تشغيلات قريبة من الانتهاء" compact />
    <div v-else class="space-y-4">
      <AppCard v-for="[supplier, group] in bySupplier" :key="supplier" :title="supplier" :subtitle="`${group.length} تشغيلة`" padding="none">
        <table class="w-full text-body">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="w-8 px-4 py-2" />
              <th class="px-2 py-2 text-start font-medium">الصنف</th>
              <th class="px-2 py-2 text-start font-medium">التشغيلة</th>
              <th class="px-2 py-2 text-start font-medium">تاريخ الصلاحية</th>
              <th class="px-2 py-2 text-start font-medium">الحالة</th>
              <th class="px-2 py-2 text-start font-medium">الكمية</th>
              <th class="px-4 py-2 text-start font-medium">القيمة</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in group" :key="r.id" class="border-b border-border last:border-0">
              <td class="px-4 py-2"><input type="checkbox" class="size-4" :checked="selected.has(r.id)" @change="toggle(r.id)" /></td>
              <td class="px-2 py-2">
                <RouterLink :to="{ name: 'product', params: { id: r.productId } }" class="hover:text-primary">{{ r.productName }}</RouterLink>
                <span class="num block text-tiny text-text-secondary">{{ r.productSku }}</span>
              </td>
              <td class="px-2 py-2 num">{{ r.batchNo }}</td>
              <td class="px-2 py-2 num">{{ formatDate(r.expiryDate) }}</td>
              <td class="px-2 py-2"><StatusBadge :tone="BUCKET_LABEL[r.bucket].tone" :label="BUCKET_LABEL[r.bucket].label" /></td>
              <td class="px-2 py-2 num">{{ formatNumber(r.qty) }}</td>
              <td class="px-4 py-2"><MoneyText :value="r.qty * r.unitCost" plain /></td>
            </tr>
          </tbody>
        </table>
      </AppCard>
    </div>

    <AppModal v-model:open="returnOpen" title="إرجاع للمورد">
      <p class="text-body leading-6">
        سيتم تسجيل <strong>مرتجع مشتريات (إشعار مدين)</strong> لـ <strong>{{ returnSupplierName ?? 'المورد' }}</strong> بالتشغيلات المحددة
        ({{ formatNumber(selectedRows.length) }} تشغيلة) — خصم من المخزون وتخفيض رصيد المورد فوراً.
      </p>
      <div class="mt-3"><AppSelect v-model="refundMethod" label="طريقة الاسترداد" :options="refundOptions" /></div>
      <p v-if="!returnSupplierName" class="mt-3 text-xs text-danger">التشغيلات المحددة تخص موردين مختلفين أو بلا مورد — حدد تشغيلات من نفس المورد.</p>
      <template #footer>
        <AppButton :disabled="busy" @click="returnOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :disabled="!returnSupplierName" :loading="busy" @click="confirmReturn">تسجيل المرتجع</AppButton>
      </template>
    </AppModal>
  </div>
</template>
