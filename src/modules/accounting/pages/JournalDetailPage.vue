<script setup lang="ts">
/**
 * Entry detail v2 (docs/v2/11-journal-dashboard-insights.md A3): audit trail, related entries
 * ("القيود المرتبطة"), attachments viewer, reversal dialog with date + required reason (B3 fix).
 */
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Copy, ExternalLink, Undo2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDateTime, todayKey } from '@/modules/core/helpers/format';
import { getCustomers, getSuppliers } from '@/modules/parties/services/partyService';
import type { Customer, Supplier } from '@/modules/parties/types';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getAccounts, getJournalEntry, reverseJournalEntry, type AccountWithBalance } from '../services/accountingService';
import type { JournalEntryType } from '../types';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
// `route.params.id` is reactive, not a static id — navigating detail→detail (e.g. after reversal,
// which pushes the reversal entry's own /accounting/journal/:id) reuses this component instance,
// so `id` must track the current route rather than being captured once at setup.
const id = computed(() => String(route.params.id));

const entry = useAsync(() => getJournalEntry(id.value));
watch(id, () => entry.reload());
const accounts = ref(new Map<string, AccountWithBalance>());
const customers = ref(new Map<string, Customer>());
const suppliers = ref(new Map<string, Supplier>());
onMounted(async () => {
  const [a, c, s] = await Promise.all([getAccounts(), getCustomers(), getSuppliers()]);
  accounts.value = new Map(a.map((x) => [x.id, x]));
  customers.value = new Map(c.map((x) => [x.id, x]));
  suppliers.value = new Map(s.map((x) => [x.id, x]));
});

const e = computed(() => entry.data.value);
const SOURCE_LABEL: Record<string, string> = {
  invoice: 'فاتورة مبيعات',
  refund: 'مرتجع مبيعات',
  purchaseOrder: 'أمر شراء',
  purchaseReturn: 'مرتجع مشتريات',
  payment: 'سند',
  stockAdjustment: 'تسوية مخزون',
};
const TYPE_LABEL: Record<JournalEntryType, string> = {
  SYSTEM: 'قيد آلي',
  MANUAL: 'قيد يدوي',
  OPENING: 'قيد افتتاحي',
  CLOSING: 'قيد إقفال',
  VAT_SETTLEMENT: 'تسوية ضريبية',
};

const canReverse = computed(
  () => auth.can('accounting', 'write') && e.value?.type === 'MANUAL' && e.value.status === 'POSTED' && !e.value.reversed && !e.value.reversalOfId,
);
const busy = ref(false);

// --- Reversal dialog (B3): date (default today, or the original date if its period is open) + required reason ---
const reverseOpen = ref(false);
const reverseDate = ref(todayKey());
const reverseReason = ref('');
const reverseSubmitted = ref(false);

function openReverseDialog() {
  reverseDate.value = todayKey();
  reverseReason.value = '';
  reverseSubmitted.value = false;
  reverseOpen.value = true;
}

async function confirmReverse() {
  reverseSubmitted.value = true;
  if (!reverseReason.value.trim() || !reverseDate.value) return;
  busy.value = true;
  try {
    const reversal = await reverseJournalEntry(id.value, new Date(reverseDate.value).toISOString(), reverseReason.value.trim());
    toast.success('تم عكس القيد', reversal.number);
    reverseOpen.value = false;
    router.push(`/accounting/journal/${reversal.id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}

function duplicateEntry() {
  router.push(`/accounting/journal/new?duplicate=${id.value}`);
}
</script>

<template>
  <div>
    <ErrorState v-if="entry.error.value" :message="entry.error.value" @retry="entry.reload" />
    <template v-else>
      <PageHeader :title="e ? `قيد ${e.number}` : '…'" back="/accounting/journal">
        <template v-if="e" #badge>
          <StatusBadge :tone="e.type === 'SYSTEM' ? 'primary' : e.type === 'CLOSING' || e.type === 'VAT_SETTLEMENT' ? 'warning' : 'neutral'" :label="TYPE_LABEL[e.type]" />
          <StatusBadge v-if="e.status === 'DRAFT'" tone="warning" label="مسودة" />
          <StatusBadge v-if="e.reversed" tone="danger" label="معكوس" />
        </template>
        <template v-if="e" #subtitle>{{ e.description }}</template>
        <template #actions>
          <AppButton v-if="e?.sourceLink" :icon="ExternalLink" :to="e.sourceLink">
            {{ SOURCE_LABEL[e.sourceRef!.kind] }} <span class="num">{{ e.sourceRef?.number }}</span>
          </AppButton>
          <AppButton v-if="e" :icon="Copy" @click="duplicateEntry">نسخ إلى قيد جديد</AppButton>
          <AppButton v-if="canReverse" variant="danger" :icon="Undo2" :loading="busy" @click="openReverseDialog">عكس القيد</AppButton>
        </template>
      </PageHeader>

      <div class="mb-4 grid gap-4 sm:grid-cols-3">
        <AppCard padding="sm">
          <p class="text-xs text-text-secondary">التاريخ</p>
          <p class="num mt-1 text-body font-medium">{{ e ? formatDateTime(e.date) : '…' }}</p>
        </AppCard>
        <AppCard padding="sm">
          <p class="text-xs text-text-secondary">أنشأه</p>
          <p class="mt-1 text-body font-medium">{{ e?.createdByName ?? '…' }}</p>
        </AppCard>
        <AppCard padding="sm">
          <p class="text-xs text-text-secondary">حالة التوازن</p>
          <p class="mt-1 text-body font-medium text-success">متوازن ✓ — المدين = الدائن</p>
        </AppCard>
      </div>

      <p v-if="e?.reversalOfId" class="mb-3 text-body text-text-secondary">
        هذا القيد يعكس
        <RouterLink :to="`/accounting/journal/${e.reversalOfId}`" class="text-primary hover:underline">القيد الأصلي</RouterLink>.
        <span v-if="e.reversalReason">السبب: {{ e.reversalReason }}</span>
      </p>
      <p v-if="e?.reversedById" class="mb-3 text-body text-text-secondary">
        تم عكس هذا القيد بالقيد
        <RouterLink :to="`/accounting/journal/${e.reversedById}`" class="num text-primary hover:underline">{{ e.reversedByNumber }}</RouterLink>.
        <span v-if="e.reversalReason">السبب: {{ e.reversalReason }}</span>
      </p>

      <div class="overflow-hidden rounded-xl border border-border">
        <div v-if="!e" class="p-4"><SkeletonBlock :lines="4" /></div>
        <table v-else class="w-full text-body">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="px-4 py-2.5 text-start font-medium">الحساب</th>
              <th class="px-3 py-2.5 text-start font-medium">البيان</th>
              <th class="px-3 py-2.5 text-start font-medium">مدين</th>
              <th class="px-4 py-2.5 text-start font-medium">دائن</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="l in e.lines" :key="l.id" class="border-b border-border last:border-0">
              <td class="px-4 py-2.5" :class="l.credit > 0 && 'ps-10'">
                <RouterLink :to="`/reports/ledger?account=${l.accountId}`" class="inline-flex items-center gap-2 hover:text-primary">
                  <span class="num text-text-secondary">{{ accounts.get(l.accountId)?.code }}</span>{{ accounts.get(l.accountId)?.name ?? '…' }}
                </RouterLink>
              </td>
              <td class="px-3 py-2.5 text-text-secondary">
                {{ l.description ?? '—' }}
                <span v-if="l.partyId" class="text-tiny">
                  — {{ (l.partyKind === 'supplier' ? suppliers : customers).get(l.partyId)?.name ?? '' }}
                </span>
              </td>
              <td class="px-3 py-2.5"><MoneyText v-if="l.debit" :value="l.debit" plain /></td>
              <td class="px-4 py-2.5"><MoneyText v-if="l.credit" :value="l.credit" plain /></td>
            </tr>
          </tbody>
          <tfoot class="border-t border-border bg-surface font-medium">
            <tr>
              <td class="px-4 py-2.5" colspan="2">الإجمالي</td>
              <td class="px-3 py-2.5"><MoneyText :value="e.totalDebit" /></td>
              <td class="px-4 py-2.5"><MoneyText :value="e.totalCredit" /></td>
            </tr>
          </tfoot>
        </table>
      </div>

      <!-- Related entries ("القيود المرتبطة") -->
      <AppCard v-if="e?.related.length" title="القيود المرتبطة" padding="sm" class="mt-4">
        <ul class="divide-y divide-border">
          <li v-for="r in e.related" :key="r.id" class="flex items-center justify-between py-2 text-body">
            <RouterLink :to="`/accounting/journal/${r.id}`" class="flex items-center gap-2 hover:text-primary">
              <span class="num font-medium">{{ r.number }}</span>
              <span class="text-text-secondary">{{ r.description }}</span>
            </RouterLink>
            <span class="num text-text-secondary">{{ formatDateTime(r.date) }}</span>
          </li>
        </ul>
      </AppCard>

      <!-- Attachments viewer (Phase 0's AttachmentField) -->
      <AppCard v-if="e" title="المرفقات" padding="sm" class="mt-4">
        <AttachmentField :owner-ref="`journal:${e.id}`" :readonly="e.status === 'POSTED'" restrict-remove />
      </AppCard>

      <!-- Audit trail -->
      <AppCard v-if="e" title="سجل العمليات" padding="sm" class="mt-4">
        <ol class="space-y-2 text-body">
          <li class="flex items-center gap-2">
            <span class="size-1.5 rounded-full bg-text-secondary" />
            <span>أُنشئ بواسطة {{ e.createdByName }} — <span class="num text-text-secondary">{{ formatDateTime(e.createdAt) }}</span></span>
          </li>
          <li v-if="e.postedAt" class="flex items-center gap-2">
            <span class="size-1.5 rounded-full bg-success" />
            <span>رُحّل — <span class="num text-text-secondary">{{ formatDateTime(e.postedAt) }}</span></span>
          </li>
          <li v-if="e.reversed" class="flex items-center gap-2">
            <span class="size-1.5 rounded-full bg-danger" />
            <span>عُكس — {{ e.reversalReason }}</span>
          </li>
        </ol>
      </AppCard>
    </template>

    <AppModal v-model:open="reverseOpen" title="عكس القيد" description="سيُنشأ قيد جديد بنفس المبالغ مع تبديل المدين والدائن. يبقى القيد الأصلي في السجل." size="sm">
      <div class="space-y-4">
        <AppInput
          v-model="reverseDate"
          type="date"
          label="تاريخ قيد العكس"
          required
          :error="reverseSubmitted && !reverseDate ? 'التاريخ مطلوب' : undefined"
        />
        <AppTextarea
          v-model="reverseReason"
          label="سبب العكس"
          required
          placeholder="مثال: خطأ في اختيار الحساب"
          :error="reverseSubmitted && !reverseReason.trim() ? 'سبب العكس مطلوب' : undefined"
        />
      </div>
      <template #footer>
        <AppButton @click="reverseOpen = false">إلغاء</AppButton>
        <AppButton variant="danger" :icon="Undo2" :loading="busy" @click="confirmReverse">عكس القيد</AppButton>
      </template>
    </AppModal>
  </div>
</template>
