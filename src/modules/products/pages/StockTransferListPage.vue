<script setup lang="ts">
/**
 * v2 phase 9 (docs/v2/07-products-and-inventory.md §4 "Branch stock & transfers", deferred from
 * phase 6): draft -> send -> receive (with shortage) / reject, all on one page. doc 17 F-1: migrated
 * onto `ListPage` + `DataTable`; the create/receive modals live in `StockTransferModals.vue` to keep
 * this page under ~250 lines (rule 12).
 */
import { onMounted, ref } from 'vue';
import { ArrowLeftRight, Check, Printer, Trash2, Truck } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import { isTauri } from '@tauri-apps/api/core';
import { useToast } from '@/modules/core/controllers/useToast';
import { renderAndSave } from '@/modules/core/services/pdfService';
import { getBranches } from '@/modules/settings/services/branchesService';
import type { Branch } from '@/modules/settings/types';
import { getProducts } from '../services/productService';
import { createTransfer, getTransfers, receiveTransfer, rejectTransfer, sendTransfer } from '../services/transferService';
import type { Product, StockTransfer } from '../types';
import StockTransferModals from '../components/StockTransferModals.vue';

const toast = useToast();
const loading = ref(true);
const transfers = ref<StockTransfer[]>([]);
const branches = ref<Branch[]>([]);
const products = ref<Product[]>([]);

const branchName = (id: string) => branches.value.find((b) => b.id === id)?.name ?? id;

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

const columns: Column<StockTransfer>[] = [
  { key: 'number', label: 'الرقم', sortable: true },
  { key: 'fromBranchId', label: 'من' },
  { key: 'toBranchId', label: 'إلى' },
  { key: 'date', label: 'التاريخ', type: 'date', sortable: true },
  { key: 'status', label: 'الحالة' },
  { key: 'actions', label: '', type: 'actions', noPrint: true },
];

const modalsRef = ref<InstanceType<typeof StockTransferModals>>();
const createOpen = ref(false);
const saving = ref(false);

function openCreate() {
  modalsRef.value?.openCreate();
}

async function saveDraft(payload: { fromBranchId: string; toBranchId: string; note: string; lines: { productId: string; qty: number }[] }) {
  if (!payload.fromBranchId || !payload.toBranchId) return toast.warning('اختر الفرع المرسل والمستقبل');
  if (payload.fromBranchId === payload.toBranchId) return toast.warning('لا يمكن التحويل لنفس الفرع');
  if (!payload.lines.length) return toast.warning('أضف صنفاً واحداً على الأقل');
  saving.value = true;
  try {
    await createTransfer({ fromBranchId: payload.fromBranchId, toBranchId: payload.toBranchId, date: new Date().toISOString(), note: payload.note || undefined, lines: payload.lines });
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
const savingReceive = ref(false);

function openReceive(t: StockTransfer) {
  receiving.value = t;
  modalsRef.value?.resetReceiveQty(t);
  receiveOpen.value = true;
}

async function confirmReceive(receiveQty: Record<string, number>) {
  if (!receiving.value) return;
  savingReceive.value = true;
  try {
    await receiveTransfer(receiving.value.id, { lines: receiving.value.lines.map((l) => ({ productId: l.productId, receivedQty: receiveQty[l.productId] ?? 0 })) });
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

/**
 * v2 phase 11b (docs/v2/12-documents-pdf-excel.md §3 "transfer note" now has a real template).
 */
async function printTransferNote(t: StockTransfer) {
  if (!isTauri()) {
    toast.info('ملف PDF الفعلي متاح في نسخة سطح المكتب');
    return;
  }
  const ok = await renderAndSave('transferNote', t.id, `${t.number}.pdf`);
  if (!ok) toast.error('تعذر إنشاء ملف PDF');
}
</script>

<template>
  <ListPage
    title="تحويلات المخزون بين الفروع"
    subtitle="مسودة ← إرسال (بضاعة بالطريق) ← استلام، مع معالجة العجز والرفض"
    primary-action-label="تحويل جديد"
    @primary-action="openCreate"
  >
    <DataTable
      :columns="columns"
      :rows="transfers"
      :loading="loading"
      :empty-icon="ArrowLeftRight"
      empty-title="لا توجد تحويلات"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-fromBranchId="{ row }">{{ branchName(row.fromBranchId) }}</template>
      <template #cell-toBranchId="{ row }">{{ branchName(row.toBranchId) }}</template>
      <template #cell-status="{ row }">
        <StatusBadge :label="STATUS_LABEL[row.status].label" :tone="STATUS_LABEL[row.status].tone" />
        <span v-if="row.shortageValue" class="ms-1.5 text-tiny text-warning">عجز <MoneyText :value="row.shortageValue" /></span>
      </template>
      <template #cell-actions="{ row }">
        <div class="flex justify-end gap-1.5" @click.stop>
          <AppButton v-if="row.status === 'DRAFT'" size="sm" variant="ghost" :icon="Truck" @click="doSend(row)">إرسال</AppButton>
          <AppButton v-if="row.status === 'SENT'" size="sm" variant="primary" :icon="Check" @click="openReceive(row)">استلام</AppButton>
          <AppButton v-if="row.status === 'SENT'" size="sm" variant="ghost" :icon="Trash2" @click="doReject(row)">رفض</AppButton>
          <AppButton v-if="row.status === 'SENT' || row.status === 'RECEIVED'" size="sm" variant="ghost" :icon="Printer" @click="printTransferNote(row)">طباعة</AppButton>
        </div>
      </template>
    </DataTable>

    <StockTransferModals
      ref="modalsRef"
      v-model:create-open="createOpen"
      v-model:receive-open="receiveOpen"
      :branches="branches"
      :products="products"
      :saving="saving"
      :saving-receive="savingReceive"
      :receiving="receiving"
      @save-draft="saveDraft"
      @confirm-receive="confirmReceive"
    />
  </ListPage>
</template>
