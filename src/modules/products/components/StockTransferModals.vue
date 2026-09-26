<script setup lang="ts">
/**
 * Extracted from StockTransferListPage (doc 17 F-1) to keep the list page under ~250 lines —
 * the "create draft" and "receive" modals for stock transfers. Pure presentation + emits; the
 * page owns data loading and service calls.
 */
import { computed, reactive, ref } from 'vue';
import { Plus, Trash2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import { useGridTab } from '@/modules/core/controllers/useGridTab';
import { formatNumber } from '@/modules/core/helpers/format';
import { toNum } from '@/modules/core/helpers/numbers';
import { branchStockQty } from '../services/transferService';
import type { Product, StockTransfer } from '../types';
import type { Branch } from '@/modules/settings/types';

const createOpen = defineModel<boolean>('createOpen', { default: false });
const receiveOpen = defineModel<boolean>('receiveOpen', { default: false });

const props = defineProps<{
  branches: Branch[];
  products: Product[];
  saving: boolean;
  savingReceive: boolean;
  receiving: StockTransfer | null;
}>();

const emit = defineEmits<{
  saveDraft: [payload: { fromBranchId: string; toBranchId: string; note: string; lines: { productId: string; qty: number }[] }];
  confirmReceive: [receiveQty: Record<string, number>];
}>();

const productName = (id: string) => props.products.find((p) => p.id === id)?.name ?? id;

// --- Create (draft) ---
interface DraftLine {
  key: number;
  productId?: string;
  qty?: number;
}
let seq = 0;
const form = reactive({ fromBranchId: '', toBranchId: '', note: '' });
const draftLines = ref<DraftLine[]>([]);

const branchOptions = computed(() => props.branches.filter((b) => b.active).map((b) => ({ value: b.id, label: b.name })));
const productOptions = computed(() =>
  props.products.map((p) => ({
    value: p.id,
    label: p.name,
    sublabel: form.fromBranchId ? `المتوفر بالفرع: ${formatNumber(branchStockQty(p.id, form.fromBranchId))}` : p.sku,
  })),
);

function openCreate() {
  Object.assign(form, { fromBranchId: props.branches[0]?.id ?? '', toBranchId: '', note: '' });
  draftLines.value = [{ key: seq++ }];
  createOpen.value = true;
}

function addLine() {
  draftLines.value.push({ key: seq++ });
}
const draftLinesEl = ref<HTMLElement>();
const onDraftLinesKeydown = useGridTab({
  container: draftLinesEl,
  rowSelector: ':scope > div',
  addRow: addLine,
  isFilled: (i) => !!draftLines.value[i]?.productId,
});
function removeLine(key: number) {
  draftLines.value = draftLines.value.filter((l) => l.key !== key);
}

function saveDraft() {
  const lines = draftLines.value.filter((l) => l.productId && toNum(l.qty)).map((l) => ({ productId: l.productId!, qty: toNum(l.qty)! }));
  emit('saveDraft', { fromBranchId: form.fromBranchId, toBranchId: form.toBranchId, note: form.note, lines });
}

// --- Receive ---
const receiveQty = reactive<Record<string, number>>({});
function resetReceiveQty(t: StockTransfer) {
  for (const k of Object.keys(receiveQty)) delete receiveQty[k];
  for (const l of t.lines) receiveQty[l.productId] = l.qty;
}
defineExpose({ openCreate, resetReceiveQty });

function confirmReceive() {
  emit('confirmReceive', { ...receiveQty });
}
</script>

<template>
  <AppModal v-model:open="createOpen" title="تحويل مخزون جديد" :persistent="saving" size="lg">
    <form class="space-y-4" novalidate @submit.prevent="saveDraft">
      <div class="grid grid-cols-2 gap-3">
        <AppSelect v-model="form.fromBranchId" label="من فرع" :options="branchOptions" />
        <AppSelect v-model="form.toBranchId" label="إلى فرع" :options="branchOptions.filter((o) => o.value !== form.fromBranchId)" />
      </div>
      <div class="space-y-2">
        <p class="text-tiny font-medium text-text-secondary">الأصناف</p>
        <div ref="draftLinesEl" class="space-y-2" @keydown="onDraftLinesKeydown">
          <div v-for="l in draftLines" :key="l.key" class="flex items-center gap-2">
            <AppCombobox v-model="l.productId" class="flex-1" placeholder="اختر منتجاً" :options="productOptions" />
            <AppInput v-model="l.qty" type="number" min="0" class="w-28" placeholder="الكمية" />
            <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" data-grid-skip @click="removeLine(l.key)">
              <Trash2 class="size-4" />
            </button>
          </div>
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
</template>
