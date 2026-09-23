<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ExternalLink, Undo2 } from '@lucide/vue';
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
import { formatDateTime } from '@/modules/core/helpers/format';
import { getCustomers, getSuppliers } from '@/modules/parties/services/partyService';
import type { Customer, Supplier } from '@/modules/parties/types';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getAccounts, getJournalEntry, reverseJournalEntry, type AccountWithBalance } from '../services/accountingService';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const id = String(route.params.id);

const entry = useAsync(() => getJournalEntry(id));
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

const canReverse = computed(
  () => auth.can('accounting', 'write') && e.value?.type === 'MANUAL' && !e.value.reversed && !e.value.reversalOfId,
);
const busy = ref(false);

async function reverse() {
  const ok = await confirm({
    title: `عكس القيد ${e.value?.number}؟`,
    message: 'سيُنشأ قيد جديد بنفس المبالغ مع تبديل المدين والدائن. يبقى القيد الأصلي في السجل.',
    confirmText: 'عكس القيد',
    danger: true,
  });
  if (!ok) return;
  busy.value = true;
  try {
    const reversal = await reverseJournalEntry(id);
    toast.success('تم عكس القيد', reversal.number);
    router.push(`/accounting/journal/${reversal.id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div>
    <ErrorState v-if="entry.error.value" :message="entry.error.value" @retry="entry.reload" />
    <template v-else>
      <PageHeader :title="e ? `قيد ${e.number}` : '…'" back="/accounting/journal">
        <template v-if="e" #badge>
          <StatusBadge :tone="e.type === 'SYSTEM' ? 'primary' : 'neutral'" :label="e.type === 'SYSTEM' ? 'قيد آلي' : 'قيد يدوي'" />
          <StatusBadge v-if="e.reversed" tone="danger" label="معكوس" />
        </template>
        <template v-if="e" #subtitle>{{ e.description }}</template>
        <template #actions>
          <AppButton v-if="e?.sourceLink" :icon="ExternalLink" :to="e.sourceLink">
            {{ SOURCE_LABEL[e.sourceRef!.kind] }} <span class="num">{{ e.sourceRef?.number }}</span>
          </AppButton>
          <AppButton v-if="canReverse" variant="danger" :icon="Undo2" :loading="busy" @click="reverse">عكس القيد</AppButton>
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
      </p>
      <p v-if="e?.reversedById" class="mb-3 text-body text-text-secondary">
        تم عكس هذا القيد بالقيد
        <RouterLink :to="`/accounting/journal/${e.reversedById}`" class="num text-primary hover:underline">{{ e.reversedByNumber }}</RouterLink>.
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
    </template>
  </div>
</template>
