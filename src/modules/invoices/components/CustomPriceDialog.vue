<script setup lang="ts">
/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §1 "Price: click to edit when pos.overridePrice allows
 * it. The floor is minPrice, or cost unless pos.sellBelowCost. A reason is required when the price
 * is below the list price."). Also carries the line's unit picker (§1 "Unit picker") and batch
 * override (§1 "Batch... can be changed") since they're all edited from the same cart-line popover.
 */
import { computed, ref, watch } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { num0, toNum } from '@/modules/core/helpers/numbers';
import { verifyManagerPin } from '@/modules/users/services/authService';
import type { CartLine } from '../controllers/usePosStore';

const props = defineProps<{
  line: CartLine | null;
  floor: number;
  sellBelowCostAllowed: boolean;
  batches?: { id: string; batchNo: string; expiryDate?: string; qty: number }[];
}>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ apply: [patch: { price: number; reason?: string; batchId?: string }] }>();

const price = ref<number | undefined>();
const reason = ref('');
const batchId = ref<string | undefined>();
const pinUser = ref('');
const pinPass = ref('');
const pinError = ref('');
const pinBusy = ref(false);
const needsApproval = ref(false);

watch(open, (o) => {
  if (o && props.line) {
    price.value = props.line.price;
    reason.value = props.line.priceOverrideReason ?? '';
    batchId.value = props.line.batchId;
    needsApproval.value = false;
    pinUser.value = '';
    pinPass.value = '';
    pinError.value = '';
  }
});

const belowList = computed(() => props.line && num0(price.value) < props.line.listPrice - 0.001);
const belowFloor = computed(() => num0(price.value) < props.floor - 0.001);
const problem = computed(() => {
  if (toNum(price.value) === undefined || num0(price.value) < 0) return 'أدخل سعراً صحيحاً';
  if (belowFloor.value && !props.sellBelowCostAllowed) return `لا يمكن البيع بأقل من ${formatNumber(props.floor)}`;
  if (belowList.value && !reason.value.trim()) return 'أدخل سبب تخفيض السعر';
  return '';
});

async function apply() {
  if (problem.value) return;
  if (belowFloor.value && props.sellBelowCostAllowed && !needsApproval.value) {
    needsApproval.value = true;
    return;
  }
  emit('apply', { price: num0(price.value), reason: belowList.value ? reason.value.trim() : undefined, batchId: batchId.value });
  open.value = false;
}

async function submitPin() {
  pinError.value = '';
  if (!pinUser.value.trim() || !pinPass.value) {
    pinError.value = 'أدخل اسم المستخدم وكلمة المرور';
    return;
  }
  pinBusy.value = true;
  try {
    await verifyManagerPin(pinUser.value, pinPass.value);
    emit('apply', { price: num0(price.value), reason: reason.value.trim() || undefined, batchId: batchId.value });
    open.value = false;
  } catch (err) {
    pinError.value = errorMessage(err);
  } finally {
    pinBusy.value = false;
  }
}
</script>

<template>
  <AppModal v-model:open="open" title="تعديل السعر" size="sm">
    <div v-if="line" class="space-y-4">
      <p class="text-body font-medium">{{ line.name }}</p>
      <div v-if="!needsApproval" class="space-y-3">
        <label class="field-label" for="custom-price">السعر</label>
        <input id="custom-price" v-model.number="price" type="number" min="0" step="0.01" class="control h-11 text-lg" autofocus @focus="($event.target as HTMLInputElement).select()" />
        <p class="text-xs text-text-secondary">السعر الأصلي: <span class="num">{{ formatNumber(line.listPrice) }}</span> — الحد الأدنى: <span class="num">{{ formatNumber(floor) }}</span></p>

        <div v-if="belowList">
          <label class="field-label" for="price-reason">سبب تخفيض السعر</label>
          <input id="price-reason" v-model="reason" class="control h-10" placeholder="مثال: عميل مميز، تلف بسيط..." />
        </div>

        <div v-if="batches?.length">
          <AppSelect v-model="batchId" label="التشغيلة" :options="batches.map((b) => ({ value: b.id, label: `${b.batchNo}${b.expiryDate ? ` — ينتهي ${b.expiryDate}` : ''} (متاح ${formatNumber(b.qty)})` }))" placeholder="تلقائي (الأقرب انتهاءً)" />
        </div>

        <p v-if="problem" class="text-xs text-danger">{{ problem }}</p>
      </div>

      <div v-else class="space-y-3">
        <p class="rounded-md bg-warning/10 px-3 py-2 text-body text-warning">السعر أقل من التكلفة — يلزم اعتماد مدير.</p>
        <input v-model="pinUser" class="control h-10" placeholder="اسم مستخدم المدير" ltr autofocus />
        <input v-model="pinPass" type="password" class="control h-10" placeholder="كلمة المرور" ltr @keydown.enter="submitPin" />
        <p v-if="pinError" class="text-xs text-danger">{{ pinError }}</p>
      </div>
    </div>
    <template #footer>
      <AppButton @click="open = false">إلغاء</AppButton>
      <AppButton v-if="!needsApproval" variant="primary" :disabled="!!problem" @click="apply">تطبيق</AppButton>
      <AppButton v-else variant="primary" :loading="pinBusy" @click="submitPin">اعتماد وتطبيق</AppButton>
    </template>
  </AppModal>
</template>
