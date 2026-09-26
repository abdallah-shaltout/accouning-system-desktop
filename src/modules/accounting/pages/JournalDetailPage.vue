<script setup lang="ts">
/**
 * Entry detail v2 (docs/v2/11-journal-dashboard-insights.md A3): audit trail, related entries
 * ("القيود المرتبطة"), attachments viewer, reversal dialog with date + required reason (B3 fix).
 */
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Copy, ExternalLink, Printer, Undo2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import { TableRow, TableCell } from '@/modules/core/components/shadcn/table';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import DetailPage, { type DetailTab } from '@/modules/core/components/layouts/DetailPage.vue';
import type { MetaChip } from '@/modules/core/components/blocks/DetailHeader.vue';
import type { StatCard } from '@/modules/core/components/blocks/StatCards.vue';
import type { Tone } from '@/modules/core/helpers/labels';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDate, formatDateTime, todayKey } from '@/modules/core/helpers/format';
import { tafqit } from '@/modules/core/helpers/tafqit';
import { getCustomers, getSuppliers } from '@/modules/parties/services/partyService';
import type { Customer, Supplier } from '@/modules/parties/types';
import ReportPrintDialog from '@/modules/reports/components/ReportPrintDialog.vue';
import { money, note, row, table as printTable } from '@/modules/reports/print/build';
import { useOfficialPrint } from '@/modules/reports/print/useOfficialPrint';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getAccounts, getJournalEntry, reverseJournalEntry, type AccountWithBalance } from '../services/accountingService';
import type { JournalEntryType } from '../types';

const route = useRoute('journal-entry');
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
  // v2 phase 8 (docs/v2/09-purchases-payments-expenses.md §4-§5, §2).
  expense: 'مصروف',
  voucher: 'سند عام',
  settlement: 'تسوية بطاقات',
  shift: 'إغلاق وردية',
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

const status = computed((): { label: string; tone: Tone } | undefined => {
  if (!e.value) return undefined;
  if (e.value.reversed) return { label: 'معكوس', tone: 'danger' };
  if (e.value.status === 'DRAFT') return { label: 'مسودة', tone: 'warning' };
  const tone: Tone = e.value.type === 'SYSTEM' ? 'primary' : e.value.type === 'CLOSING' || e.value.type === 'VAT_SETTLEMENT' ? 'warning' : 'neutral';
  return { label: TYPE_LABEL[e.value.type], tone };
});

const chips = computed<MetaChip[]>(() => (e.value ? [{ label: 'البيان', value: e.value.description }] : []));

const stats = computed<StatCard[]>(() => {
  if (!e.value) return [];
  return [
    { label: 'التاريخ', value: formatDateTime(e.value.date) },
    { label: 'أنشأه', value: e.value.createdByName },
    { label: 'حالة التوازن', value: 'متوازن ✓ — المدين = الدائن' },
  ];
});

const tabs = computed<DetailTab[]>(() => {
  const t: DetailTab[] = [{ key: 'lines', label: 'بنود القيد' }];
  if (e.value?.related.length) t.push({ key: 'related', label: 'القيود المرتبطة' });
  t.push({ key: 'attachments', label: 'المرفقات' });
  t.push({ key: 'history', label: 'سجل العمليات' });
  return t;
});

type JournalLine = NonNullable<typeof e.value>['lines'][number];
const lineColumns: Column<JournalLine>[] = [
  { key: 'accountId', label: 'الحساب' },
  { key: 'description', label: 'البيان' },
  { key: 'debit', label: 'مدين', type: 'money' },
  { key: 'credit', label: 'دائن', type: 'money' },
];

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
    router.push({ name: 'journal-entry', params: { id: reversal.id } });
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}

function duplicateEntry() {
  router.push({ name: 'journal-new', query: { duplicate: id.value } });
}

// --- Official print: a journal voucher (سند قيد) rendered as its own document, never the screen ---
const { open: printOpen, doc: printDoc, show: showPrint } = useOfficialPrint();

/** "فقط … لا غير" — tafqit only closes with "لا غير" when there are no halalas. */
function amountInWords(amount: number): string {
  const words = tafqit(amount, { prefix: 'فقط' });
  return words.endsWith('لا غير') ? words : `${words} لا غير`;
}

function printEntry() {
  const v = e.value;
  if (!v) return;
  const partyName = (l: (typeof v.lines)[number]) => (l.partyId ? (l.partyKind === 'supplier' ? suppliers : customers).value.get(l.partyId)?.name : undefined);
  const m = (n: number) => money(n, { dashZero: true });
  showPrint({
    title: 'قيد يومية',
    subtitle: v.description,
    badge: TYPE_LABEL[v.type],
    userLabel: 'طُبع بواسطة',
    leadMeta: [
      { label: 'رقم القيد', value: v.number },
      { label: 'التاريخ', value: formatDate(v.date) },
      { label: 'الحالة', value: v.status === 'DRAFT' ? 'مسودة' : v.reversed ? 'مرحّل — معكوس' : 'مرحّل' },
      ...(v.sourceRef ? [{ label: 'المستند المصدر', value: `${SOURCE_LABEL[v.sourceRef.kind] ?? v.sourceRef.kind} ${v.sourceRef.number}` }] : []),
      { label: 'أنشأه', value: v.createdByName },
    ],
    signatures: true,
    signatureTitles: ['أعدّه', 'راجعه', 'اعتمده'],
    blocks: [
      ...(v.reversalOfId ? [note(`هذا القيد يعكس قيداً سابقاً${v.reversalReason ? ` — السبب: ${v.reversalReason}` : ''}`, 'warn')] : []),
      ...(v.reversedById ? [note(`تم عكس هذا القيد بالقيد ${v.reversedByNumber ?? ''}${v.reversalReason ? ` — السبب: ${v.reversalReason}` : ''}`, 'warn')] : []),
      printTable(
        [
          { label: 'الرمز', dim: true, width: 0.7 },
          { label: 'الحساب', width: 2.2 },
          { label: 'البيان', width: 2.4 },
          { label: 'مدين', numeric: true, width: 1.2 },
          { label: 'دائن', numeric: true, width: 1.2 },
        ],
        [
          ...v.lines.map((l) => {
            const acc = accounts.value.get(l.accountId);
            return row([acc?.code ?? '', acc?.name ?? l.accountId, [l.description, partyName(l)].filter(Boolean).join(' — '), m(l.debit), m(l.credit)]);
          }),
          row(['', 'الإجمالي', '', money(v.totalDebit), money(v.totalCredit)], 'total'),
        ],
      ),
      note(`المبلغ كتابةً: ${amountInWords(v.totalDebit)}`),
    ],
  });
}

// The command palette's "طباعة هذا القيد" sets `?print=1`.
watch(
  () => [e.value, route.query.print] as const,
  ([v, flag]) => {
    if (!v || flag !== '1') return;
    void router.replace({ query: { ...route.query, print: undefined } });
    printEntry();
  },
  { immediate: true },
);
</script>

<template>
  <div>
    <ErrorState v-if="entry.error.value" :message="entry.error.value" @retry="entry.reload" />
    <DetailPage
      v-else
      :title="e ? `قيد ${e.number}` : '…'"
      :status="status"
      :chips="chips"
      :back="{ name: 'journal' }"
      :stats="stats"
      :tabs="tabs"
    >
      <template #actions>
        <AppButton v-if="e?.sourceLink" :icon="ExternalLink" :to="e.sourceLink">
          {{ SOURCE_LABEL[e.sourceRef!.kind] }} <span class="num">{{ e.sourceRef?.number }}</span>
        </AppButton>
        <AppButton v-if="e" :icon="Printer" data-testid="journal-print" @click="printEntry">طباعة</AppButton>
        <AppButton v-if="e" :icon="Copy" @click="duplicateEntry">نسخ إلى قيد جديد</AppButton>
        <AppButton v-if="canReverse" variant="danger" :icon="Undo2" :loading="busy" @click="openReverseDialog">عكس القيد</AppButton>
      </template>

      <template #tab-lines>
        <p v-if="e?.reversalOfId" class="mb-3 text-body text-text-secondary">
          هذا القيد يعكس
          <RouterLink :to="{ name: 'journal-entry', params: { id: e.reversalOfId } }" class="text-primary hover:underline">القيد الأصلي</RouterLink>.
          <span v-if="e.reversalReason">السبب: {{ e.reversalReason }}</span>
        </p>
        <p v-if="e?.reversedById" class="mb-3 text-body text-text-secondary">
          تم عكس هذا القيد بالقيد
          <RouterLink :to="{ name: 'journal-entry', params: { id: e.reversedById } }" class="num text-primary hover:underline">{{ e.reversedByNumber }}</RouterLink>.
          <span v-if="e.reversalReason">السبب: {{ e.reversalReason }}</span>
        </p>

        <div class="overflow-hidden rounded-xl border border-border">
          <div v-if="!e" class="p-4"><SkeletonBlock :lines="4" /></div>
          <DataTable v-else :columns="lineColumns" :rows="e.lines" row-key="id" :page-size="0">
            <template #cell-accountId="{ row }">
              <RouterLink :to="{ name: 'report-ledger', query: { account: row.accountId } }" class="inline-flex items-center gap-2 hover:text-primary" :class="row.credit > 0 && 'ps-10'">
                <span class="num text-text-secondary">{{ accounts.get(row.accountId)?.code }}</span>{{ accounts.get(row.accountId)?.name ?? '…' }}
              </RouterLink>
            </template>
            <template #cell-description="{ row }">
              {{ row.description ?? '—' }}
              <span v-if="row.partyId" class="text-tiny">
                — {{ (row.partyKind === 'supplier' ? suppliers : customers).get(row.partyId)?.name ?? '' }}
              </span>
            </template>
            <template #cell-debit="{ row }"><MoneyText v-if="row.debit" :value="row.debit" plain /></template>
            <template #cell-credit="{ row }"><MoneyText v-if="row.credit" :value="row.credit" plain /></template>
            <template #footer>
              <TableRow>
                <TableCell class="px-4 py-2.5" colspan="2">الإجمالي</TableCell>
                <TableCell class="px-3 py-2.5"><MoneyText :value="e.totalDebit" /></TableCell>
                <TableCell class="px-4 py-2.5"><MoneyText :value="e.totalCredit" /></TableCell>
              </TableRow>
            </template>
          </DataTable>
        </div>
      </template>

      <!-- Related entries ("القيود المرتبطة") -->
      <template v-if="e?.related.length" #tab-related>
        <AppCard padding="sm">
          <ul class="divide-y divide-border">
            <li v-for="r in e.related" :key="r.id" class="flex items-center justify-between py-2 text-body">
              <RouterLink :to="{ name: 'journal-entry', params: { id: r.id } }" class="flex items-center gap-2 hover:text-primary">
                <span class="num font-medium">{{ r.number }}</span>
                <span class="text-text-secondary">{{ r.description }}</span>
              </RouterLink>
              <span class="num text-text-secondary">{{ formatDateTime(r.date) }}</span>
            </li>
          </ul>
        </AppCard>
      </template>

      <!-- Attachments viewer (Phase 0's AttachmentField) -->
      <template #tab-attachments>
        <AppCard v-if="e" padding="sm">
          <AttachmentField :owner-ref="`journal:${e.id}`" :readonly="e.status === 'POSTED'" restrict-remove />
        </AppCard>
      </template>

      <!-- Audit trail -->
      <template #tab-history>
        <AppCard v-if="e" padding="sm">
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
    </DetailPage>

    <AppModal v-model:open="reverseOpen" title="عكس القيد" description="سيُنشأ قيد جديد بنفس المبالغ مع تبديل المدين والدائن. يبقى القيد الأصلي في السجل." size="sm">
      <div class="space-y-4">
        <AppDatePicker
          v-model="reverseDate"
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

    <ReportPrintDialog v-model:open="printOpen" :doc="printDoc" :file-name="e ? `قيد ${e.number}` : 'قيد'" />
  </div>
</template>
