<script setup lang="ts">
/**
 * Payment detail with its allocation grid (docs/v2/09-purchases-payments-expenses.md §3): shows
 * what the payment has already been applied to, and — when it still has unallocated money — lets
 * the user allocate more to open documents right here ("allocate later" from docs/v2/08 §3
 * "المدفوعات… Allocate unallocated credit to open invoices from here").
 */
import { computed, reactive, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { Trash2, Wand2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDate, formatDateTime } from '@/modules/core/helpers/format';
import { PAYMENT_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { num0 } from '@/modules/core/helpers/numbers';
import { round2 } from '@/modules/invoices/helpers/totals';
import type { AppRoute } from '@/modules/core/types/route';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { allocateExistingPayment, getOpenDocuments, getPayment, removeAllocation } from '../services/paymentService';
import type { OpenDocument, PaymentAllocationInput } from '../types';
import { allocationStatusFor } from '../types';

const route = useRoute('payment-detail');
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();

const id = String(route.params.id);
const payment = useAsync(() => getPayment(id));
const docs = ref<OpenDocument[]>([]);
const docsLoading = ref(false);
const draft = reactive<Record<string, number | undefined>>({});
const saving = ref(false);

const p = computed(() => payment.data.value);
const statusTone = computed(() => {
  const s = p.value ? allocationStatusFor(p.value.amount, p.value.allocated) : 'unallocated';
  return s === 'full' ? 'success' : s === 'partial' ? 'warning' : 'neutral';
});
const statusLabel = computed(() => {
  const s = p.value ? allocationStatusFor(p.value.amount, p.value.allocated) : 'unallocated';
  return s === 'full' ? 'مخصص بالكامل' : s === 'partial' ? 'مخصص جزئياً' : 'غير مخصص';
});

async function loadOpenDocs() {
  if (!p.value) return;
  docsLoading.value = true;
  try {
    const all = await getOpenDocuments(p.value.targetType, p.value.targetId);
    const alreadyAllocated = new Set(p.value.allocations.map((a) => a.targetId));
    docs.value = all.filter((d) => !alreadyAllocated.has(d.id));
  } finally {
    docsLoading.value = false;
  }
}

async function refresh() {
  await payment.reload();
  for (const k of Object.keys(draft)) delete draft[k];
  await loadOpenDocs();
}

watch(p, (v) => {
  if (v) loadOpenDocs();
});

const draftTotal = computed(() => Object.values(draft).reduce((a: number, v) => a + num0(v), 0));
const remaining = computed(() => (p.value ? Math.max(0, round2(p.value.unallocated - draftTotal.value)) : 0));

function toggleDoc(doc: OpenDocument, checked: boolean) {
  if (checked) draft[doc.id] = Math.min(doc.outstanding, Math.max(0, round2((p.value?.unallocated ?? 0) - draftTotal.value)) || doc.outstanding);
  else delete draft[doc.id];
}

function autoAllocate() {
  for (const k of Object.keys(draft)) delete draft[k];
  let remain = p.value?.unallocated ?? 0;
  for (const doc of docs.value) {
    if (remain <= 0.005) break;
    const take = Math.min(doc.outstanding, remain);
    draft[doc.id] = round2(take);
    remain = round2(remain - take);
  }
}

async function saveAllocations() {
  if (!p.value) return;
  const inputs: PaymentAllocationInput[] = docs.value.filter((d) => num0(draft[d.id]) > 0).map((d) => ({ targetKind: d.kind, targetId: d.id, amount: round2(num0(draft[d.id])) }));
  if (!inputs.length) return;
  saving.value = true;
  try {
    await allocateExistingPayment(p.value.id, inputs);
    toast.success('تم تخصيص المبلغ');
    await refresh();
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function unallocate(allocationId: string, number: string) {
  if (!p.value) return;
  const ok = await confirm({ title: 'إلغاء التخصيص', message: `إلغاء تخصيص هذا المبلغ عن ${number}؟ يعود المبلغ رصيداً غير مخصص على السند — لا يتغير القيد المحاسبي.` });
  if (!ok) return;
  try {
    await removeAllocation(p.value.id, allocationId);
    toast.success('تم إلغاء التخصيص');
    await refresh();
  } catch (err) {
    toast.error(err);
  }
}

function docLink(kind: 'invoice' | 'purchaseOrder' | 'opening', id: string): AppRoute | undefined {
  if (kind === 'invoice') return { name: 'invoice', params: { id } };
  if (kind === 'purchaseOrder') return { name: 'purchase', params: { id } };
  return undefined;
}
</script>

<template>
  <div>
    <ErrorState v-if="payment.error.value" :message="payment.error.value" @retry="payment.reload" />
    <template v-else>
      <PageHeader :title="p ? `سند ${p.number}` : '…'" :back="{ name: 'payments' }">
        <template v-if="p" #subtitle>
          {{ p.type === 'RECEIVED' ? 'قبض من' : 'صرف إلى' }}
          <RouterLink :to="p.targetType === 'customer' ? { name: 'customer', params: { id: p.targetId } } : { name: 'supplier', params: { id: p.targetId } }" class="text-primary hover:underline">{{ p.partyName }}</RouterLink>
          — {{ formatDateTime(p.date) }}
        </template>
      </PageHeader>

      <div v-if="!p"><SkeletonBlock :lines="6" /></div>
      <div v-else class="grid items-start gap-5 lg:grid-cols-[280px_1fr]">
        <div class="space-y-4">
          <AppCard padding="sm">
            <p class="text-xs text-text-secondary">المبلغ</p>
            <p class="mt-1 text-2xl font-semibold"><MoneyText :value="p.amount" /></p>
            <div class="mt-3 space-y-1.5 border-t border-border pt-3 text-xs">
              <div class="flex items-center justify-between"><span class="text-text-secondary">الطريقة</span><span>{{ PAYMENT_METHOD_LABEL[p.method] }}</span></div>
              <div class="flex items-center justify-between"><span class="text-text-secondary">مخصص</span><span class="num">{{ p.allocated.toFixed(2) }}</span></div>
              <div class="flex items-center justify-between"><span class="text-text-secondary">غير مخصص</span><span class="num font-medium" :class="p.unallocated > 0.005 && 'text-warning'">{{ p.unallocated.toFixed(2) }}</span></div>
              <div class="flex items-center justify-between pt-1"><StatusBadge :tone="statusTone" :label="statusLabel" /></div>
            </div>
            <p v-if="p.note" class="mt-3 border-t border-border pt-3 text-xs text-text-secondary">{{ p.note }}</p>
          </AppCard>
        </div>

        <div class="space-y-5">
          <AppCard title="المستندات المخصص عليها هذا السند" padding="none">
            <EmptyState v-if="!p.allocations.length" compact title="لا يوجد تخصيص بعد" description="المبلغ بالكامل غير مخصص — يظهر كرصيد على الطرف" />
            <table v-else class="w-full text-body">
              <thead class="border-b border-border text-xs text-text-secondary">
                <tr>
                  <th class="px-3 py-2 text-start">المستند</th>
                  <th class="px-2 py-2 text-start">التاريخ</th>
                  <th class="px-2 py-2 text-end">المبلغ المخصص</th>
                  <th v-if="auth.can('payments', 'write')" class="w-10 px-3 py-2"></th>
                </tr>
              </thead>
              <tbody class="divide-y divide-border">
                <tr v-for="a in p.allocations" :key="a.id">
                  <td class="px-3 py-2">
                    <RouterLink v-if="docLink(a.targetKind, a.targetId)" :to="docLink(a.targetKind, a.targetId)!" class="num text-primary hover:underline">{{ a.targetNumber }}</RouterLink>
                    <span v-else class="num">{{ a.targetNumber }}</span>
                  </td>
                  <td class="num px-2 py-2 text-text-secondary">{{ formatDate(a.date) }}</td>
                  <td class="px-2 py-2 text-end"><MoneyText :value="a.amount" plain class="font-medium" /></td>
                  <td v-if="auth.can('payments', 'write')" class="px-3 py-2 text-end">
                    <button type="button" class="rounded p-1 text-text-secondary hover:bg-surface-hover hover:text-danger" title="إلغاء التخصيص" @click="unallocate(a.id, a.targetNumber)">
                      <Trash2 class="size-3.5" />
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </AppCard>

          <AppCard v-if="p.unallocated > 0.005 && auth.can('payments', 'write')" :title="`تخصيص الرصيد غير المخصص (${p.unallocated.toFixed(2)})`" padding="none">
            <template v-if="docs.length" #actions>
              <AppButton size="sm" :icon="Wand2" @click="autoAllocate">تخصيص تلقائي</AppButton>
            </template>
            <div v-if="docsLoading" class="p-4"><SkeletonBlock :lines="2" height="h-8" /></div>
            <EmptyState v-else-if="!docs.length" compact title="لا توجد مستندات مفتوحة أخرى لهذا الطرف حالياً" />
            <template v-else>
              <table class="w-full text-body">
                <thead class="border-b border-border text-xs text-text-secondary">
                  <tr>
                    <th class="w-8 px-3 py-2"></th>
                    <th class="px-2 py-2 text-start">المستند</th>
                    <th class="px-2 py-2 text-end">المتبقي</th>
                    <th class="w-32 px-3 py-2 text-end">تخصيص</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-border">
                  <tr v-for="d in docs" :key="d.id">
                    <td class="px-3 py-2">
                      <input type="checkbox" class="size-4 accent-primary" :checked="draft[d.id] !== undefined" @change="toggleDoc(d, ($event.target as HTMLInputElement).checked)" />
                    </td>
                    <td class="num px-2 py-2 font-medium">{{ d.number }}</td>
                    <td class="px-2 py-2 text-end"><MoneyText :value="d.outstanding" plain class="text-warning" /></td>
                    <td class="px-3 py-2">
                      <input
                        type="number"
                        min="0"
                        :max="d.outstanding"
                        class="control num h-8 w-full text-end"
                        :value="draft[d.id] ?? ''"
                        :disabled="draft[d.id] === undefined"
                        @input="draft[d.id] = num0(($event.target as HTMLInputElement).value)"
                      />
                    </td>
                  </tr>
                </tbody>
              </table>
              <div class="flex items-center justify-between border-t border-border px-4 py-2.5 text-xs">
                <span class="text-text-secondary">سيبقى غير مخصص <span class="num font-medium">{{ remaining.toFixed(2) }}</span></span>
                <AppButton size="sm" variant="primary" :loading="saving" :disabled="draftTotal <= 0" @click="saveAllocations">حفظ التخصيص</AppButton>
              </div>
            </template>
          </AppCard>
        </div>
      </div>
    </template>
  </div>
</template>
