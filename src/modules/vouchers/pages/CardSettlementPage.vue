<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { CreditCard, Landmark } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, formatDate, todayKey } from '@/modules/core/helpers/format';
import { num0 } from '@/modules/core/helpers/numbers';
import { round2 } from '@/modules/invoices/helpers/totals';
import { createCardSettlement, estimateSettlementFee, getUnsettledTenderGroups } from '../services/voucherService';
import type { UnsettledTenderGroup } from '../types';

const router = useRouter();
const toast = useToast();

const groups = ref<UnsettledTenderGroup[]>([]);
const loading = ref(true);
const selected = ref<Set<string>>(new Set());
const date = ref(todayKey());
const depositAmount = ref<number>();
const note = ref('');
const saving = ref(false);
const feeEstimated = ref(false);

function key(g: UnsettledTenderGroup) {
  return `${g.date}::${g.paymentMethodId}`;
}

async function load() {
  loading.value = true;
  try {
    groups.value = await getUnsettledTenderGroups();
  } finally {
    loading.value = false;
  }
}
onMounted(load);

const selectedGroups = computed(() => groups.value.filter((g) => selected.value.has(key(g))));
const grossTotal = computed(() => round2(selectedGroups.value.reduce((a, g) => a + g.total, 0)));
const feeAmount = computed(() => round2(grossTotal.value - num0(depositAmount.value)));

function toggle(g: UnsettledTenderGroup) {
  const k = key(g);
  const next = new Set(selected.value);
  if (next.has(k)) next.delete(k);
  else next.add(k);
  selected.value = next;
  feeEstimated.value = false;
}

// Pre-fill the deposit amount from the fee-% estimate, once, when the selection changes (docs/v2/09
// §2 "fee = the difference, pre-filled from fee %").
watch(selectedGroups, async (list) => {
  if (!list.length) {
    depositAmount.value = undefined;
    return;
  }
  const estFee = await estimateSettlementFee(list);
  depositAmount.value = round2(grossTotal.value - estFee);
  feeEstimated.value = true;
});

async function submit() {
  if (!selectedGroups.value.length) return;
  saving.value = true;
  try {
    const settlement = await createCardSettlement({
      date: dateKeyToIso(date.value),
      groups: selectedGroups.value.map((g) => ({ date: g.date, paymentMethodId: g.paymentMethodId })),
      depositAmount: num0(depositAmount.value),
      note: note.value.trim() || undefined,
    });
    toast.success('تم تسجيل تسوية البطاقات', settlement.number);
    router.push('/payments');
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="تسوية البطاقات والمحافظ" subtitle="اختر العمليات غير المسواة، أدخل مبلغ الإيداع البنكي، والعمولة = الفرق" />
    <div class="grid items-start gap-5 xl:grid-cols-[1fr_320px]">
      <AppCard padding="none">
        <SkeletonBlock v-if="loading" :lines="6" height="h-10" class="p-4" />
        <EmptyState v-else-if="!groups.length" :icon="CreditCard" title="لا توجد عمليات غير مسواة" compact />
        <table v-else class="w-full text-body">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="w-8 px-4 py-2" />
              <th class="px-2 py-2 text-start font-medium">التاريخ</th>
              <th class="px-2 py-2 text-start font-medium">الطريقة</th>
              <th class="px-2 py-2 text-start font-medium">عدد العمليات</th>
              <th class="px-4 py-2 text-start font-medium">الإجمالي</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="g in groups" :key="key(g)" class="border-b border-border last:border-0">
              <td class="px-4 py-2"><input type="checkbox" class="size-4" :checked="selected.has(key(g))" @change="toggle(g)" /></td>
              <td class="num px-2 py-2">{{ formatDate(g.date) }}</td>
              <td class="px-2 py-2">{{ g.paymentMethodName }}</td>
              <td class="num px-2 py-2 text-text-secondary">{{ g.tenderCount }}</td>
              <td class="px-4 py-2"><MoneyText :value="g.total" plain /></td>
            </tr>
          </tbody>
        </table>
      </AppCard>

      <AppCard title="التسوية" padding="sm">
        <div class="space-y-4">
          <AppInput v-model="date" type="date" label="تاريخ الإيداع" />
          <div class="flex items-center justify-between text-body">
            <span class="text-text-secondary">إجمالي المحدد</span>
            <MoneyText :value="grossTotal" />
          </div>
          <AppInput v-model.number="depositAmount" type="number" min="0" step="0.01" label="مبلغ الإيداع البنكي" :hint="feeEstimated ? 'مُقترح تلقائياً حسب نسبة العمولة — يمكن تعديله' : undefined" />
          <div class="flex items-center justify-between text-body">
            <span class="text-text-secondary">العمولة (الفرق)</span>
            <MoneyText :value="feeAmount" :class="feeAmount < 0 && 'text-danger'" />
          </div>
          <AppInput v-model="note" label="ملاحظات" />
          <AppButton variant="primary" block :icon="Landmark" :disabled="!selectedGroups.length" :loading="saving" @click="submit">ترحيل التسوية</AppButton>
          <p class="text-tiny leading-5 text-text-secondary">القيد: من البنك ومن عمولات البطاقات، إلى حساب تسوية البطاقات/المحافظ.</p>
        </div>
      </AppCard>
    </div>
  </div>
</template>
