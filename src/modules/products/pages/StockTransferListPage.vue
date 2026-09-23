<script setup lang="ts">
/**
 * v2 phase 9 (docs/v2/07-products-and-inventory.md §4 "Branch stock & transfers", deferred from
 * phase 6): draft -> send -> receive (with shortage) / reject, all on one page — list + a create
 * modal + a receive modal, since the flow is short enough not to need separate routes.
 */
import { computed, onMounted, reactive, ref } from 'vue';
import { ArrowLeftRight, Check, Plus, Trash2, Truck } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { toNum } from '@/modules/core/helpers/numbers';
import { getBranches } from '@/modules/settings/services/branchesService';
import type { Branch } from '@/modules/settings/types';
import { getProducts } from '../services/productService';
import { branchStockQty, createTransfer, getTransfers, receiveTransfer, rejectTransfer, sendTransfer } from '../services/transferService';
import type { Product, StockTransfer } from '../types';

const toast = useToast();
const loading = ref(true);
const transfers = ref<StockTransfer[]>([]);
const branches = ref<Branch[]>([]);
const products = ref<Product[]>([]);

const branchName = (id: string) => branches.value.find((b) => b.id === id)?.name ?? id;
const productName = (id: string) => products.value.find((p) => p.id === id)?.name ?? id;

const STATUS_LABEL: Record<StockTransfer['status'], { label: string; tone: 'neutral' | 'success' | 'warning' | 'danger' | 'primary' }> = {
  DRAFT: { label: 'مسودة', tone: 'neutral' },
  SENT: { label: 'مُرسل — بالطريق', tone: 'warning' },
  RECEIVED: { label: 'مستلم', tone: 'success' },
  REJECTED: { label: 'مرفوض', tone: 'danger' },
};

async function reload() {
  transfers.value = (await getTransfers()).slice().sort((a, b) => b.date.localeCompare(a.date));
}

onMounted(async () => {
  const [b, p] = await Promise.all([getBranches(), getProducts({ type: 'product' })]);
  branches.value = b;
  products.value = p;
  await reload();
  loading.value = false;
});

// --- Create (draft) ---
const createOpen = ref(false);
const saving = ref(false);
interface DraftLine {
  key: number;
  productId?: string;
  qty?: number;
}
let seq = 0;
const form = reactive({ fromBranchId: '', toBranchId: '', note: '' });
const draftLines = ref<DraftLine[]>([]);

const branchOptions = computed(() => branches.value.filter((b) => b.active).map((b) => ({ value: b.id, label: b.name })));
const productOptions = computed(() =>
  products.value.map((p) => ({
    value: p.id,
    label: p.name,
    sublabel: form.fromBranchId ? `المتوفر بالفرع: ${formatNumber(branchStockQty(p.id, form.fromBranchId))}` : p.sku,
  })),
);

function openCreate() {
  Object.assign(form, { fromBranchId: branches.value[0]?.id ?? '', toBranchId: '', note: '' });
  draftLines.value = [{ key: seq++ }];
  createOpen.value = true;
}

function addLine() {
  draftLines.value.push({ key: seq++ });
}
function removeLine(key: number) {
  draftLines.value = draftLines.value.filter((l) => l.key !== key);
}

async function saveDraft() {
  if (!form.fromBranchId || !form.toBranchId) return toast.warning('اختر الفرع المرسل والمستقبل');
  if (form.fromBranchId === form.toBranchId) return toast.warning('لا يمكن التحويل لنفس الفرع');
  const lines = draftLines.value.filter((l) => l.productId && toNum(l.qty)).map((l) => ({ productId: l.productId!, qty: toNum(l.qty)! }));
  if (!lines.length) return toast.warning('أضف صنفاً واحداً على الأقل');
  saving.value = true;
  try {
    await createTransfer({ fromBranchId: form.fromBranchId, toBranchId: form.toBranchId, date: new Date().toISOString(), note: form.note || undefined, lines });
    await reload();
    toast.success('تم إنشاء مسودة التحويل');
    createOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function doSend(t: StockTransfer) {
  try {
    await sendTransfer(t.id);
    await reload();
    toast.success(`تم إرسال التحويل ${t.number}`);
  } catch (err) {
    toast.error(err);
  }
}

// --- Receive ---
const receiveOpen = ref(false);
const receiving = ref<StockTransfer | null>(null);
const receiveQty = reactive<Record<string, number>>({});
const savingReceive = ref(false);

function openReceive(t: StockTransfer) {
  receiving.value = t;
  for (const l of t.lines) receiveQty[l.productId] = l.qty;
  receiveOpen.value = true;
}

async function confirmReceive() {
  if (!receiving.value) return;
  savingReceive.value = true;
  try {
    await receiveTransfer(
      receiving.value.id,
      { lines: receiving.value.lines.map((l) => ({ productId: l.productId, receivedQty: toNum(receiveQty[l.productId]) ?? 0 })) },
    );
    await reload();
    toast.success('تم استلام التحويل');
    receiveOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    savingReceive.value = false;
  }
}

async function doReject(t: StockTransfer) {
  const reason = window.prompt('سبب الرفض؟');
  if (!reason?.trim()) return;
  try {
    await rejectTransfer(t.id, reason.trim());
    await reload();
    toast.success('تم رفض التحويل');
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="تحويلات المخزون بين الفروع" subtitle="مسودة ← إرسال (بضاعة بالطريق) ← استلام، مع معالجة العجز والرفض" />

    <SkeletonBlock v-if="loading" :lines="4" height="h-14" />
    <AppCard v-else padding="none">
      <template #actions>
        <AppButton size="sm" :icon="Plus" @click="openCreate">تحويل جديد</AppButton>
      </template>
      <EmptyState v-if="!transfers.length" title="لا توجد تحويلات" :icon="ArrowLeftRight" />
      <table v-else class="w-full text-body">
        <thead class="text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="px-4 py-2 text-start font-medium">الرقم</th>
            <th class="px-2 py-2 text-start font-medium">من</th>
            <th class="px-2 py-2 text-start font-medium">إلى</th>
            <th class="px-2 py-2 text-start font-medium">التاريخ</th>
            <th class="px-2 py-2 text-start font-medium">الحالة</th>
            <th class="px-4 py-2"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="t in transfers" :key="t.id" class="border-b border-border last:border-0">
            <td class="px-4 py-2 num font-medium">{{ t.number }}</td>
            <td class="px-2 py-2">{{ branchName(t.fromBranchId) }}</td>
            <td class="px-2 py-2">{{ branchName(t.toBranchId) }}</td>
            <td class="px-2 py-2 num text-text-secondary">{{ formatDateTime(t.date) }}</td>
            <td class="px-2 py-2">
              <StatusBadge :label="STATUS_LABEL[t.status].label" :tone="STATUS_LABEL[t.status].tone" />
              <span v-if="t.shortageValue" class="ms-1.5 text-tiny text-warning">عجز <MoneyText :value="t.shortageValue" /></span>
            </td>
            <td class="px-4 py-2 text-end">
              <div class="flex justify-end gap-1.5">
                <AppButton v-if="t.status === 'DRAFT'" size="sm" variant="ghost" :icon="Truck" @click="doSend(t)">إرسال</AppButton>
                <AppButton v-if="t.status === 'SENT'" size="sm" variant="primary" :icon="Check" @click="openReceive(t)">استلام</AppButton>
                <AppButton v-if="t.status === 'SENT'" size="sm" variant="ghost" :icon="Trash2" @click="doReject(t)">رفض</AppButton>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </AppCard>

    <AppModal v-model:open="createOpen" title="تحويل مخزون جديد" :persistent="saving" size="lg">
      <form class="space-y-4" novalidate @submit.prevent="saveDraft">
        <div class="grid grid-cols-2 gap-3">
          <AppSelect v-model="form.fromBranchId" label="من فرع" :options="branchOptions" />
          <AppSelect v-model="form.toBranchId" label="إلى فرع" :options="branchOptions.filter((o) => o.value !== form.fromBranchId)" />
        </div>
        <div class="space-y-2">
          <p class="text-tiny font-medium text-text-secondary">الأصناف</p>
          <div v-for="l in draftLines" :key="l.key" class="flex items-center gap-2">
            <AppCombobox v-model="l.productId" class="flex-1" placeholder="اختر منتجاً" :options="productOptions" />
            <AppInput v-model="l.qty" type="number" min="0" class="w-28" placeholder="الكمية" />
            <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" @click="removeLine(l.key)">
              <Trash2 class="size-4" />
            </button>
          </div>
          <AppButton size="sm" variant="ghost" :icon="Plus" @click="addLine">إضافة صنف</AppButton>
        </div>
        <AppTextarea v-model="form.note" label="ملاحظة" :rows="2" />
      </form>
      <template #footer>
        <AppButton :disabled="saving" @click="createOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="saving" @click="saveDraft">حفظ كمسودة</AppButton>
      </template>
    </AppModal>

    <AppModal v-model:open="receiveOpen" :title="`استلام التحويل ${receiving?.number ?? ''}`" :persistent="savingReceive">
      <div v-if="receiving" class="space-y-3">
        <p class="text-tiny text-text-secondary">عدّل الكمية المستلمة إن اختلفت عن المرسلة — الفرق يُرحّل تلقائياً كعجز على فروقات جرد المخزون.</p>
        <div v-for="l in receiving.lines" :key="l.productId" class="flex items-center justify-between gap-3">
          <span class="flex-1 truncate text-body">{{ productName(l.productId) }} <span class="num text-tiny text-text-secondary">(أُرسل {{ formatNumber(l.qty) }})</span></span>
          <AppInput v-model="receiveQty[l.productId]" type="number" min="0" :max="l.qty" class="w-28" />
        </div>
      </div>
      <template #footer>
        <AppButton :disabled="savingReceive" @click="receiveOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="savingReceive" @click="confirmReceive">تأكيد الاستلام</AppButton>
      </template>
    </AppModal>
  </div>
</template>
