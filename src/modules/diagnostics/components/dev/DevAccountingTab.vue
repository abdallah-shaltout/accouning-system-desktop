<script setup lang="ts">
/**
 * "المحاسبة" tab (18.F3) inside `/dev/diagnostics` — picks a posted document, then shows its
 * posting trace, raw journal lines, account resolution, balances before/after, plus a
 * document-independent invariants panel and a subledger-vs-GL drift report. Dev-only, read-only:
 * everything here comes from `accountingDebugService.ts` (seam-safe — never imports `src/mocks/*`
 * directly). Answers the plan's open question ("does `/dev/diagnostics`'s Accounting tab stub
 * host this, or is it a separate route?") — it hosts it; no new `/dev/accounting` route was added
 * since this tab, reachable the same way, already existed for exactly this purpose.
 */
import { computed, onMounted, ref, watch } from 'vue';
import { FlaskConical, GitCompareArrows } from '@lucide/vue';
import AppCombobox, { type ComboOption } from '@/modules/core/components/ui/AppCombobox.vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/modules/core/components/shadcn/tabs';
import { formatDate } from '@/modules/core/helpers/format';
import {
  explainAccountBalance,
  exportReproBundle,
  getBalancesAround,
  getDriftReport,
  getInvariantResults,
  getJournalEntryRaw,
  getPostingTrace,
  isReproRecording,
  listRecentDocuments,
  startReproRecording,
  stopReproRecording,
  type AccountingDocSummary,
  type DriftRow,
  type ExplainLine,
} from '../../services/accountingDebugService';
import type { InvariantResult } from '@/mocks';
import type { PostingTrace } from '@/mocks';
import { useToast } from '@/modules/core/controllers/useToast';

const recording = ref(false);
onMounted(async () => {
  recording.value = await isReproRecording();
});

async function toggleRecording() {
  if (recording.value) {
    await stopReproRecording();
  } else {
    await startReproRecording();
  }
  recording.value = await isReproRecording();
}

async function exportBundle() {
  const exported = await exportReproBundle();
  if (!exported) {
    useToast().info('لا توجد حالة مسجّلة بعد', 'ابدأ التسجيل أولاً ثم كرّر الخطوات المسبّبة للمشكلة.');
  }
}

const documents = ref<AccountingDocSummary[]>([]);
const selectedId = ref<string>();
const loadingDocs = ref(true);

const trace = ref<PostingTrace | undefined>();
const rawEntry = ref<Awaited<ReturnType<typeof getJournalEntryRaw>>>();
const balancesAround = ref<Awaited<ReturnType<typeof getBalancesAround>>>([]);
const loadingDoc = ref(false);

const invariants = ref<InvariantResult[]>([]);
const loadingInvariants = ref(true);

const drift = ref<DriftRow[]>([]);
const loadingDrift = ref(true);

const explainAccountId = ref<string>();
const explainRows = ref<ExplainLine[]>([]);
const loadingExplain = ref(false);

onMounted(async () => {
  documents.value = await listRecentDocuments();
  loadingDocs.value = false;
  invariants.value = await getInvariantResults();
  loadingInvariants.value = false;
  drift.value = await getDriftReport();
  loadingDrift.value = false;
  if (documents.value.length) selectedId.value = documents.value[0].id;
});

const docOptions = computed<ComboOption[]>(() =>
  documents.value.map((d) => ({
    value: d.id,
    label: `${d.number} — ${d.description}`,
    sublabel: `${formatDate(d.date)} · ${d.type}${d.sourceKind ? ` · ${d.sourceKind}` : ''}`,
  })),
);

watch(selectedId, async (id) => {
  if (!id) return;
  loadingDoc.value = true;
  trace.value = await getPostingTrace(id);
  rawEntry.value = await getJournalEntryRaw(id);
  balancesAround.value = await getBalancesAround(id);
  loadingDoc.value = false;
});

async function runExplain(accountId: string) {
  explainAccountId.value = accountId;
  loadingExplain.value = true;
  explainRows.value = await explainAccountBalance(accountId);
  loadingExplain.value = false;
}

const lineColumns: Column<{ id: string; accountId: string; description?: string; debit: number; credit: number; partyId?: string }>[] = [
  { key: 'accountId', label: 'الحساب' },
  { key: 'description', label: 'البيان' },
  { key: 'debit', label: 'مدين', type: 'money' },
  { key: 'credit', label: 'دائن', type: 'money' },
];

const balanceColumns: Column<{ accountId: string; accountName: string; accountCode?: string; before: number; after: number }>[] = [
  { key: 'accountCode', label: 'الرمز' },
  { key: 'accountName', label: 'الحساب' },
  { key: 'before', label: 'قبل', type: 'money' },
  { key: 'after', label: 'بعد', type: 'money' },
];

const invariantColumns: Column<InvariantResult>[] = [
  { key: 'doc', label: 'البند' },
  { key: 'key', label: 'المفتاح' },
  { key: 'passed', label: 'الحالة' },
  { key: 'message', label: 'التفاصيل' },
];

const driftColumns: Column<DriftRow>[] = [
  { key: 'kind', label: 'النوع' },
  { key: 'label', label: 'الاسم' },
  { key: 'subledger', label: 'دفتر مساعد', type: 'money' },
  { key: 'gl', label: 'الأستاذ العام', type: 'money' },
  { key: 'diff', label: 'الفرق', type: 'money' },
  { key: 'firstDivergingDocNumber', label: 'أول مستند مختلف' },
];

const explainColumns: Column<ExplainLine>[] = [
  { key: 'docNumber', label: 'المستند' },
  { key: 'docDate', label: 'التاريخ' },
  { key: 'sourceKind', label: 'النوع' },
  { key: 'description', label: 'البيان' },
  { key: 'debit', label: 'مدين', type: 'money' },
  { key: 'credit', label: 'دائن', type: 'money' },
];

function stepKindLabel(kind: string): string {
  return (
    {
      lineDiscount: 'خصم السطر',
      invoiceDiscount: 'توزيع خصم الفاتورة',
      vat: 'الضريبة',
      accountResolution: 'اختيار الحساب',
      cost: 'التكلفة',
      fx: 'تحويل العملة',
      note: 'ملاحظة',
    }[kind] ?? kind
  );
}
</script>

<template>
  <div class="flex flex-col gap-6">
    <section class="flex flex-col gap-3 rounded-xl border border-border p-4">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <div class="flex items-center gap-2">
          <FlaskConical class="size-4 text-text-secondary" />
          <h3 class="text-body-sm font-medium">مصحح الحسابات — اختر مستنداً</h3>
        </div>
        <div class="flex items-center gap-2">
          <AppButton :variant="recording ? 'danger' : 'secondary'" size="sm" @click="toggleRecording">
            {{ recording ? 'إيقاف تسجيل إعادة الإنتاج' : 'بدء تسجيل إعادة الإنتاج' }}
          </AppButton>
          <AppButton variant="ghost" size="sm" @click="exportBundle">تصدير حالة لإعادة الإنتاج</AppButton>
        </div>
      </div>
      <AppCombobox v-model="selectedId" :options="docOptions" placeholder="اختر قيداً…" search-placeholder="بحث برقم القيد أو البيان…" />

      <Tabs v-if="selectedId" default-value="trace">
        <TabsList>
          <TabsTrigger value="trace">تتبع الترحيل</TabsTrigger>
          <TabsTrigger value="lines">سطور القيد</TabsTrigger>
          <TabsTrigger value="accounts">اختيار الحسابات</TabsTrigger>
          <TabsTrigger value="balances">الأرصدة قبل/بعد</TabsTrigger>
        </TabsList>

        <TabsContent value="trace">
          <div v-if="loadingDoc" class="py-8 text-center text-text-secondary">جارِ التحميل…</div>
          <div v-else-if="!trace" class="rounded-lg border border-dashed border-border py-8 text-center text-text-secondary">
            لا يوجد تتبع محفوظ لهذا القيد (فُعّل وضع التتبع بعد ترحيله، أو انتهت صلاحية الذاكرة المؤقتة).
          </div>
          <ol v-else class="flex flex-col gap-2">
            <li v-for="(step, i) in trace.steps" :key="i" class="rounded-lg border border-border p-3">
              <div class="flex items-center justify-between">
                <span class="text-body-sm font-medium">{{ i + 1 }}. {{ stepKindLabel(step.kind) }} — {{ step.label }}</span>
              </div>
              <pre class="num mt-1 overflow-x-auto text-xs text-text-secondary">{{ JSON.stringify(step.detail, null, 2) }}</pre>
            </li>
            <li v-if="!trace.steps.length" class="text-body-sm text-text-secondary">لا توجد خطوات تفصيلية مسجّلة لهذا القيد (لم يُرفق المستدعي أي شروحات).</li>
          </ol>
        </TabsContent>

        <TabsContent value="lines">
          <DataTable :columns="lineColumns" :rows="rawEntry?.lines ?? []" :loading="loadingDoc" empty-title="لا توجد سطور">
            <template #cell-accountId="{ row }"><span class="num text-xs">{{ row.accountId }}</span></template>
          </DataTable>
        </TabsContent>

        <TabsContent value="accounts">
          <ul class="flex flex-col gap-2">
            <li v-for="(step, i) in trace?.steps.filter((s) => s.kind === 'accountResolution')" :key="i" class="rounded-lg border border-border p-3 text-body-sm">
              <span class="num">{{ step.detail.chosenAccountId }}</span>
              <span v-if="step.detail.role" class="text-text-secondary"> — دور: {{ step.detail.role }}</span>
              <p class="mt-1 text-xs text-text-secondary">{{ step.detail.why }}</p>
              <AppButton variant="ghost" size="sm" class="mt-1" @click="runExplain(String(step.detail.chosenAccountId))">اشرح هذا الرقم</AppButton>
            </li>
            <li v-if="!trace?.steps.some((s) => s.kind === 'accountResolution')" class="text-body-sm text-text-secondary">لا توجد بيانات اختيار حسابات لهذا القيد.</li>
          </ul>
        </TabsContent>

        <TabsContent value="balances">
          <DataTable :columns="balanceColumns" :rows="balancesAround" :loading="loadingDoc" empty-title="لا توجد حسابات">
            <template #cell-accountName="{ row }">
              <button type="button" class="text-primary underline-offset-2 hover:underline" @click="runExplain(row.accountId)">{{ row.accountName }}</button>
            </template>
          </DataTable>
        </TabsContent>
      </Tabs>
    </section>

    <section v-if="explainAccountId" class="flex flex-col gap-2 rounded-xl border border-border p-4">
      <h3 class="text-body-sm font-medium">اشرح هذا الرقم — <span class="num">{{ explainAccountId }}</span></h3>
      <DataTable :columns="explainColumns" :rows="explainRows" :loading="loadingExplain" empty-title="لا توجد سطور لهذا الحساب">
        <template #cell-docDate="{ row }"><span class="num text-xs">{{ formatDate(row.docDate) }}</span></template>
      </DataTable>
    </section>

    <section class="flex flex-col gap-2 rounded-xl border border-border p-4">
      <h3 class="text-body-sm font-medium">القواعد المحاسبية (invariants)</h3>
      <DataTable :columns="invariantColumns" :rows="invariants" :loading="loadingInvariants" empty-title="لا توجد نتائج">
        <template #cell-passed="{ row }"><StatusBadge :tone="row.passed ? 'success' : 'danger'" :label="row.passed ? 'ناجح' : 'فشل'" /></template>
      </DataTable>
    </section>

    <section class="flex flex-col gap-2 rounded-xl border border-border p-4">
      <div class="flex items-center gap-2">
        <GitCompareArrows class="size-4 text-text-secondary" />
        <h3 class="text-body-sm font-medium">تقرير الانحراف — دفتر مساعد مقابل الأستاذ العام</h3>
      </div>
      <DataTable :columns="driftColumns" :rows="drift" :loading="loadingDrift" empty-title="لا يوجد انحراف — كل الأرصدة متطابقة">
        <template #cell-firstDivergingDocNumber="{ row }">
          <button v-if="row.firstDivergingDocId" type="button" class="num text-primary underline-offset-2 hover:underline" @click="selectedId = row.firstDivergingDocId">
            {{ row.firstDivergingDocNumber }}
          </button>
          <span v-else class="text-text-secondary">—</span>
        </template>
      </DataTable>
    </section>
  </div>
</template>
