<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { BookText } from '@lucide/vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { formatDate, formatDateTime } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { banner, count, kpis, money, row, table as printTable } from '../print/build';
import type { ReportPrintSpec } from '../print/types';
import { getAccountLedger, getLedgerTargets, getPartyLedger } from '../services/reportService';
import type { AccountLedger } from '../types';

type Kind = 'account' | 'customer' | 'supplier';

const route = useRoute();
const router = useRouter();
const { from, to, fiscalStart, ready } = useReportRange();

const initialKind: Kind = route.query.customer ? 'customer' : route.query.supplier ? 'supplier' : 'account';
const kind = ref<Kind>(initialKind);
const targetId = ref<string | undefined>((route.query[initialKind] as string | undefined) ?? undefined);
const targets = ref<Awaited<ReturnType<typeof getLedgerTargets>>>();
const data = ref<AccountLedger>();
const loading = ref(false);
const error = ref<string | null>(null);

onMounted(async () => (targets.value = await getLedgerTargets()));

const options = computed(() => {
  const list = kind.value === 'account' ? targets.value?.accounts : kind.value === 'customer' ? targets.value?.customers : targets.value?.suppliers;
  return (list ?? []).map((t) => ({ value: t.id, label: t.label }));
});

async function load() {
  router.replace({ query: { [kind.value]: targetId.value, from: from.value || undefined, to: to.value || undefined } });
  if (!targetId.value) {
    data.value = undefined;
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    const range = { from: from.value || undefined, to: to.value || undefined };
    data.value = kind.value === 'account' ? await getAccountLedger(targetId.value, range) : await getPartyLedger(kind.value, targetId.value, range);
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}

watch(kind, () => {
  targetId.value = undefined;
  data.value = undefined;
});
watch([targetId, from, to, ready], () => ready.value && load());

const table = computed<ExportTable | undefined>(() => {
  const d = data.value;
  if (!d) return undefined;
  return {
    title: `كشف حساب — ${d.title}`,
    columns: ['التاريخ', 'المستند', 'البيان', 'مدين', 'دائن', 'الرصيد'],
    rows: [
      ['', '', 'رصيد أول المدة', '', '', d.openingBalance],
      ...d.rows.map((r) => [formatDateTime(r.date), r.entryNumber, r.description, r.debit, r.credit, r.balance]),
      ['', '', 'الإجمالي / رصيد آخر المدة', d.totalDebit, d.totalCredit, d.closingBalance],
    ],
  };
});

// Official print (reference `generalLedger.hbs`: account banner, balance bar, running-balance table).
const print = computed<ReportPrintSpec | null>(() => {
  const d = data.value;
  if (!d) return null;
  const m = (v: number) => money(v, { dashZero: true });
  return {
    title: kind.value === 'account' ? 'كشف حساب — دفتر الأستاذ' : kind.value === 'customer' ? 'كشف حساب عميل' : 'كشف حساب مورد',
    subtitle: 'الحركات المرحّلة خلال الفترة مع الرصيد التراكمي',
    signatures: true,
    blocks: [
      banner(d.title, d.subtitle, `طبيعة الرصيد: ${d.normalSide === 'DEBIT' ? 'مدين' : 'دائن'}`),
      kpis([
        { label: 'رصيد أول المدة', value: money(d.openingBalance) },
        { label: 'إجمالي المدين', value: money(d.totalDebit) },
        { label: 'إجمالي الدائن', value: money(d.totalCredit) },
        { label: 'عدد الحركات', value: count(d.rows.length) },
        { label: 'رصيد آخر المدة', value: money(d.closingBalance), emphasis: true },
      ]),
      printTable(
        [
          { label: 'التاريخ', dim: true, width: 1 },
          { label: 'المستند', width: 1.1 },
          { label: 'البيان', width: 3 },
          { label: 'مدين', numeric: true, width: 1.2 },
          { label: 'دائن', numeric: true, width: 1.2 },
          { label: 'الرصيد', numeric: true, width: 1.3 },
        ],
        [
          row(['', '', 'رصيد أول المدة', '', '', money(d.openingBalance)], 'opening'),
          ...d.rows.map((r) => row([formatDate(r.date), r.entryNumber, r.description, m(r.debit), m(r.credit), money(r.balance)])),
          row(['', '', 'الإجمالي / رصيد آخر المدة', money(d.totalDebit), money(d.totalCredit), money(d.closingBalance)], 'total'),
        ],
      ),
    ],
  };
});
</script>

<template>
  <ReportShell
    :title="data ? `كشف حساب — ${data.title}` : 'كشف حساب'"
    :subtitle="data?.subtitle ?? 'اختر حساباً أو عميلاً أو مورداً'"
    :from="from"
    :to="to"
    :loading="loading && !data"
    :error="error"
    :table="table"
    :print="print"
    @retry="load"
  >
    <template #filters>
      <SegmentedControl
        v-model="kind"
        :options="[
          { value: 'account', label: 'حساب' },
          { value: 'customer', label: 'عميل' },
          { value: 'supplier', label: 'مورد' },
        ]"
      />
      <AppCombobox v-model="targetId" class="w-72" :options="options" :placeholder="kind === 'account' ? 'اختر الحساب…' : kind === 'customer' ? 'اختر العميل…' : 'اختر المورد…'" />
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <EmptyState v-if="!targetId" :icon="BookText" title="اختر ما تريد عرض كشف حسابه" description="حساب من دليل الحسابات، أو عميل، أو مورد" />
    <div v-else-if="data" class="overflow-hidden rounded-xl border border-border" :class="loading && 'opacity-60'">
      <table class="w-full text-body">
        <thead class="bg-surface text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="px-3 py-2.5 text-start font-medium">التاريخ</th>
            <th class="px-3 py-2.5 text-start font-medium">المستند</th>
            <th class="px-3 py-2.5 text-start font-medium">البيان</th>
            <th class="px-3 py-2.5 text-start font-medium">مدين</th>
            <th class="px-3 py-2.5 text-start font-medium">دائن</th>
            <th class="px-3 py-2.5 text-start font-medium">الرصيد</th>
          </tr>
        </thead>
        <tbody>
          <tr class="border-b border-border bg-surface/50">
            <td class="px-3 py-2" colspan="5">رصيد أول المدة</td>
            <td class="px-3 py-2"><MoneyText :value="data.openingBalance" plain class="font-medium" /></td>
          </tr>
          <tr v-for="r in data.rows" :key="r.id" class="border-b border-border last:border-0 hover:bg-surface-hover">
            <td class="px-3 py-2"><span class="num text-text-secondary">{{ formatDateTime(r.date) }}</span></td>
            <td class="px-3 py-2">
              <RouterLink v-if="data.rowLinks[r.id]" :to="data.rowLinks[r.id]" class="num text-primary hover:underline">{{ r.entryNumber }}</RouterLink>
            </td>
            <td class="px-3 py-2">{{ r.description }}</td>
            <td class="px-3 py-2"><MoneyText :value="r.debit" plain dash-zero /></td>
            <td class="px-3 py-2"><MoneyText :value="r.credit" plain dash-zero /></td>
            <td class="px-3 py-2"><MoneyText :value="r.balance" plain :class="r.balance < 0 && 'text-danger'" /></td>
          </tr>
          <tr v-if="!data.rows.length">
            <td colspan="6" class="px-3 py-6 text-center text-xs text-text-secondary">لا توجد حركات في هذه الفترة</td>
          </tr>
        </tbody>
        <tfoot class="border-t border-border bg-surface font-medium">
          <tr>
            <td class="px-3 py-2.5" colspan="3">الإجمالي / رصيد آخر المدة</td>
            <td class="px-3 py-2.5"><MoneyText :value="data.totalDebit" plain /></td>
            <td class="px-3 py-2.5"><MoneyText :value="data.totalCredit" plain /></td>
            <td class="px-3 py-2.5"><MoneyText :value="data.closingBalance" /></td>
          </tr>
        </tfoot>
      </table>
    </div>
  </ReportShell>
</template>
