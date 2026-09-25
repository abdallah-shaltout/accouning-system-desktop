<script setup lang="ts">
/**
 * Journal list v2 (docs/v2/11-journal-dashboard-insights.md A1): day-grouped with sticky headers +
 * daily Dr/Cr totals, inline row expansion, full filters, saved views (localStorage — same pattern
 * as Phase 0's appearance settings, since there's no per-user record in the mock backend), footer
 * totals, Excel export, and a day-book PDF — which opens the reports hub's day book with its
 * official print preview (letterhead, entry bands, signatures; `modules/reports/print/`).
 */
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { Bookmark, ChevronDown, FileDown, Paperclip, Plus, Printer, Wallet, X } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import DirIcon from '@/modules/core/components/ui/DirIcon.vue';
import { dirIcon } from '@/modules/core/helpers/dirIcon';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useHotkeys } from '@/modules/core/controllers/useHotkeys';
import { useToast } from '@/modules/core/controllers/useToast';
import { exportXlsx } from '@/modules/core/helpers/exportXlsx';
import { formatDateLong, formatDateTime, formatNumber, daysAgoKey, todayKey } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { accountPath, getAccounts, getJournalEntries, type AccountWithBalance, type JournalRow } from '../services/accountingService';
import type { JournalEntryType, JournalFilter, JournalSavedView } from '../types';

const router = useRouter();
const auth = useAuthStore();
const toast = useToast();

const type = ref<'all' | JournalEntryType>('all');
const status = ref<'all' | 'DRAFT' | 'POSTED'>('all');
const search = ref('');
const from = ref(daysAgoKey(29));
const to = ref(todayKey());
const accountId = ref<string | undefined>();
const hasAttachments = ref(false);
const reversedOnly = ref(false);
const minAmount = ref<number | undefined>();
const maxAmount = ref<number | undefined>();
const filtersOpen = ref(false);

const accounts = ref<AccountWithBalance[]>([]);
onMounted(async () => (accounts.value = await getAccounts()));
const accountOptions = computed(() =>
  accounts.value.filter((a) => !a.isGroup).map((a) => ({ value: a.id, label: `${a.code} — ${a.name}`, sublabel: accountPath(a, accounts.value), keywords: a.code })),
);

const filter = computed<JournalFilter>(() => ({
  type: type.value === 'all' ? undefined : type.value,
  status: status.value === 'all' ? undefined : status.value,
  from: from.value || undefined,
  to: to.value || undefined,
  search: search.value || undefined,
  accountId: accountId.value,
  hasAttachments: hasAttachments.value || undefined,
  reversed: reversedOnly.value || undefined,
  minAmount: minAmount.value,
  maxAmount: maxAmount.value,
}));

const { data, loading, error, reload } = useAsync(() => getJournalEntries(filter.value));
watch(filter, reload, { deep: true });

const typeOptions = computed(() => [
  { value: 'all' as const, label: 'الكل', count: data.value?.length },
  { value: 'SYSTEM' as const, label: 'آلية', count: data.value?.filter((e) => e.type === 'SYSTEM').length },
  { value: 'MANUAL' as const, label: 'يدوية', count: data.value?.filter((e) => e.type === 'MANUAL').length },
  { value: 'OPENING' as const, label: 'افتتاحية', count: data.value?.filter((e) => e.type === 'OPENING').length },
  { value: 'CLOSING' as const, label: 'إقفال', count: data.value?.filter((e) => e.type === 'CLOSING').length },
  { value: 'VAT_SETTLEMENT' as const, label: 'تسوية ضريبية', count: data.value?.filter((e) => e.type === 'VAT_SETTLEMENT').length },
]);

// --- Saved views (per user, localStorage — same pattern as useAppearance.ts) -------------------

const SAVED_VIEWS_KEY = 'app_journal_saved_views';
function loadSavedViews(): JournalSavedView[] {
  try {
    const raw = localStorage.getItem(SAVED_VIEWS_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}
const savedViews = ref<JournalSavedView[]>(loadSavedViews());
function persistSavedViews() {
  try {
    localStorage.setItem(SAVED_VIEWS_KEY, JSON.stringify(savedViews.value));
  } catch {
    /* private mode — won't persist */
  }
}

// The default "مسودات بحاجة لترحيل" (drafts to post) view for accountants — always available, not stored.
const DRAFTS_VIEW_ID = '__drafts__';
function applyDraftsView() {
  type.value = 'all';
  status.value = 'DRAFT';
  from.value = '';
  to.value = '';
  accountId.value = undefined;
  hasAttachments.value = false;
  reversedOnly.value = false;
  minAmount.value = undefined;
  maxAmount.value = undefined;
  activeViewId.value = DRAFTS_VIEW_ID;
}

const activeViewId = ref<string | null>(null);
function applySavedView(view: JournalSavedView) {
  type.value = view.filter.type ?? 'all';
  status.value = view.filter.status ?? 'all';
  from.value = view.filter.from ?? '';
  to.value = view.filter.to ?? '';
  search.value = view.filter.search ?? '';
  accountId.value = view.filter.accountId;
  hasAttachments.value = !!view.filter.hasAttachments;
  reversedOnly.value = !!view.filter.reversed;
  minAmount.value = view.filter.minAmount;
  maxAmount.value = view.filter.maxAmount;
  activeViewId.value = view.id;
}
watch([type, status, from, to, search, accountId, hasAttachments, reversedOnly, minAmount, maxAmount], () => {
  if (activeViewId.value) activeViewId.value = null;
});

const saveViewModalOpen = ref(false);
const newViewName = ref('');
function saveCurrentAsView() {
  if (!newViewName.value.trim()) return;
  const view: JournalSavedView = { id: `view-${Date.now()}`, name: newViewName.value.trim(), filter: filter.value };
  savedViews.value = [...savedViews.value, view];
  persistSavedViews();
  activeViewId.value = view.id;
  newViewName.value = '';
  saveViewModalOpen.value = false;
  toast.success('تم حفظ العرض');
}
function deleteSavedView(id: string) {
  savedViews.value = savedViews.value.filter((v) => v.id !== id);
  persistSavedViews();
  if (activeViewId.value === id) activeViewId.value = null;
}

function clearFilters() {
  type.value = 'all';
  status.value = 'all';
  search.value = '';
  from.value = '';
  to.value = '';
  accountId.value = undefined;
  hasAttachments.value = false;
  reversedOnly.value = false;
  minAmount.value = undefined;
  maxAmount.value = undefined;
  activeViewId.value = null;
}

// --- Day grouping (A1) --------------------------------------------------------------------------

interface DayGroup {
  key: string;
  label: string;
  entries: JournalRow[];
  debit: number;
  credit: number;
}

const groups = computed<DayGroup[]>(() => {
  const rows = data.value ?? [];
  const byDay = new Map<string, JournalRow[]>();
  for (const e of rows) {
    const key = e.date.slice(0, 10);
    byDay.set(key, [...(byDay.get(key) ?? []), e]);
  }
  return [...byDay.entries()]
    .sort((a, b) => b[0].localeCompare(a[0]))
    .map(([key, entries]) => ({
      key,
      label: formatDateLong(entries[0].date),
      entries,
      debit: entries.reduce((a, e) => a + e.totalDebit, 0),
      credit: entries.reduce((a, e) => a + e.totalCredit, 0),
    }));
});

const footerTotals = computed(() => ({
  debit: (data.value ?? []).reduce((a, e) => a + e.totalDebit, 0),
  credit: (data.value ?? []).reduce((a, e) => a + e.totalCredit, 0),
}));

// --- Inline row expansion --------------------------------------------------------------------

const expanded = ref<Set<string>>(new Set());
function toggleExpand(id: string) {
  const next = new Set(expanded.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  expanded.value = next;
}

function accountLabel(id: string) {
  const a = accounts.value.find((x) => x.id === id);
  return a ? `${a.code} — ${a.name}` : id;
}

const TYPE_LABEL: Record<JournalEntryType, string> = {
  SYSTEM: 'آلي',
  MANUAL: 'يدوي',
  OPENING: 'افتتاحي',
  CLOSING: 'إقفال',
  VAT_SETTLEMENT: 'تسوية ضريبية',
};
const TYPE_TONE: Record<JournalEntryType, 'primary' | 'neutral' | 'success' | 'warning'> = {
  SYSTEM: 'primary',
  MANUAL: 'neutral',
  OPENING: 'success',
  CLOSING: 'warning',
  VAT_SETTLEMENT: 'warning',
};

// --- Keyboard: J/K row navigation, Enter opens, N creates (A1) ---------------------------------

const flatRows = computed(() => groups.value.flatMap((g) => g.entries));
const activeIndex = ref(-1);
function openActive() {
  const row = flatRows.value[activeIndex.value];
  if (row) openEntry(row);
}
function openEntry(row: JournalRow) {
  if (row.status === 'DRAFT') router.push(`/accounting/journal/new?draft=${row.id}`);
  else router.push(`/accounting/journal/${row.id}`);
}
useHotkeys({
  j: () => {
    activeIndex.value = Math.min(activeIndex.value + 1, flatRows.value.length - 1);
    return false;
  },
  k: () => {
    activeIndex.value = Math.max(activeIndex.value - 1, 0);
    return false;
  },
  Enter: () => {
    if (activeIndex.value >= 0) openActive();
  },
  n: () => {
    if (auth.can('accounting', 'write')) router.push('/accounting/journal/new');
    return false;
  },
});

// --- Export -------------------------------------------------------------------------------------

const exporting = ref(false);
async function exportEntries() {
  exporting.value = true;
  try {
    await exportXlsx({
      fileName: `القيود-اليومية-${from.value || 'الكل'}-${to.value || todayKey()}`,
      columns: [
        { key: 'number', label: 'رقم القيد' },
        { key: 'date', label: 'التاريخ', value: (r: JournalRow) => formatDateTime(r.date) },
        { key: 'description', label: 'البيان' },
        { key: 'type', label: 'النوع', value: (r: JournalRow) => TYPE_LABEL[r.type] },
        { key: 'status', label: 'الحالة', value: (r: JournalRow) => (r.status === 'DRAFT' ? 'مسودة' : 'مرحّل') },
        { key: 'totalDebit', label: 'مدين', numeric: true },
        { key: 'totalCredit', label: 'دائن', numeric: true },
        { key: 'createdByName', label: 'المستخدم' },
      ],
      rows: data.value ?? [],
    });
  } finally {
    exporting.value = false;
  }
}

/** Exports the entries WITH their lines — one row per line (A1's "entries, or entries with lines"). */
async function exportEntriesWithLines() {
  exporting.value = true;
  try {
    const rows = (data.value ?? []).flatMap((e) => e.lines.map((l) => ({ entry: e, line: l })));
    await exportXlsx({
      fileName: `القيود-اليومية-تفصيلي-${from.value || 'الكل'}-${to.value || todayKey()}`,
      columns: [
        { key: 'number', label: 'رقم القيد', value: (r: any) => r.entry.number },
        { key: 'date', label: 'التاريخ', value: (r: any) => formatDateTime(r.entry.date) },
        { key: 'account', label: 'الحساب', value: (r: any) => accountLabel(r.line.accountId) },
        { key: 'lineDesc', label: 'بيان السطر', value: (r: any) => r.line.description ?? '' },
        { key: 'debit', label: 'مدين', numeric: true, value: (r: any) => r.line.debit },
        { key: 'credit', label: 'دائن', numeric: true, value: (r: any) => r.line.credit },
      ],
      rows,
    });
  } finally {
    exporting.value = false;
  }
}

/** Opens the day-book report for the same period with its official print preview already open. */
function openDayBook() {
  void router.push({ path: '/reports/day-book', query: { from: from.value || undefined, to: to.value || undefined, print: '1' } });
}
</script>

<template>
  <div>
    <PageHeader title="القيود اليومية" subtitle="القيود الآلية الناتجة عن المبيعات والمشتريات والسندات والمخزون، والقيود اليدوية">
      <template #actions>
        <AppButton :icon="Printer" @click="openDayBook">دفتر اليومية PDF</AppButton>
        <AppButton :icon="FileDown" :loading="exporting" @click="exportEntries">تصدير</AppButton>
        <AppButton variant="ghost" :loading="exporting" @click="exportEntriesWithLines">تصدير بالأسطر</AppButton>
        <AppButton v-if="auth.can('accounting', 'write')" variant="primary" :icon="Plus" to="/accounting/journal/new" kbd="N">قيد يدوي</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <div class="flex flex-wrap items-center gap-2">
        <SegmentedControl v-model="type" :options="typeOptions" />
        <SearchInput v-model="search" placeholder="رقم القيد، البيان، أو المستند" />
        <AppButton size="sm" variant="ghost" :icon="ChevronDown" @click="filtersOpen = !filtersOpen">فلاتر إضافية</AppButton>
      </div>
      <DateRangeFilter v-model:from="from" v-model:to="to" />
    </div>

    <!-- Saved views -->
    <div class="mb-3 flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="inline-flex h-7 items-center gap-1.5 rounded-full border px-2.5 text-xs transition-colors"
        :class="activeViewId === DRAFTS_VIEW_ID ? 'border-primary/40 bg-primary/10 text-primary' : 'border-border text-text-secondary hover:bg-surface-hover'"
        @click="applyDraftsView"
      >
        <Bookmark class="size-3" /> مسودات بحاجة لترحيل
      </button>
      <button
        v-for="v in savedViews"
        :key="v.id"
        type="button"
        class="group inline-flex h-7 items-center gap-1.5 rounded-full border px-2.5 text-xs transition-colors"
        :class="activeViewId === v.id ? 'border-primary/40 bg-primary/10 text-primary' : 'border-border text-text-secondary hover:bg-surface-hover'"
        @click="applySavedView(v)"
      >
        {{ v.name }}
        <X class="size-3 opacity-0 group-hover:opacity-100" @click.stop="deleteSavedView(v.id)" />
      </button>
      <button type="button" class="inline-flex h-7 items-center gap-1 rounded-full px-2 text-xs text-text-secondary hover:bg-surface-hover" @click="saveViewModalOpen = true">
        <Bookmark class="size-3" /> حفظ العرض الحالي
      </button>
      <button
        v-if="activeViewId || search || accountId || hasAttachments || reversedOnly || minAmount || maxAmount || status !== 'all'"
        type="button"
        class="text-xs text-text-secondary underline hover:text-text-primary"
        @click="clearFilters"
      >
        مسح الفلاتر
      </button>
    </div>

    <div v-if="filtersOpen" class="mb-4 grid gap-3 rounded-xl border border-border p-3 sm:grid-cols-4">
      <AppCombobox v-model="accountId" :options="accountOptions" label="الحساب (يشمل الفروع)" placeholder="كل الحسابات" clearable dense />
      <div>
        <label class="field-label">الحالة</label>
        <SegmentedControl
          v-model="status"
          :options="[
            { value: 'all', label: 'الكل' },
            { value: 'DRAFT', label: 'مسودة' },
            { value: 'POSTED', label: 'مرحّل' },
          ]"
        />
      </div>
      <AppInput v-model.number="minAmount" type="number" label="من مبلغ" />
      <AppInput v-model.number="maxAmount" type="number" label="إلى مبلغ" />
      <label class="flex items-center gap-2 text-body">
        <input v-model="hasAttachments" type="checkbox" class="size-4 rounded border-border" /> يحتوي مرفقات
      </label>
      <label class="flex items-center gap-2 text-body">
        <input v-model="reversedOnly" type="checkbox" class="size-4 rounded border-border" /> معكوس فقط
      </label>
    </div>

    <div v-if="error" class="rounded-xl border border-danger/30 bg-danger/5 p-4 text-body text-danger">
      {{ error }}
      <button type="button" class="ms-2 underline" @click="reload">إعادة المحاولة</button>
    </div>
    <div v-else-if="loading && !data" class="space-y-3">
      <div v-for="i in 3" :key="i" class="h-32 animate-shimmer rounded-xl bg-surface-hover" />
    </div>
    <div v-else-if="!groups.length" class="flex flex-col items-center justify-center rounded-xl border border-border py-16 text-text-secondary">
      <Wallet class="mb-3 size-8" />
      <p>لا توجد قيود في هذه الفترة</p>
    </div>
    <div v-else class="space-y-4">
      <div v-for="group in groups" :key="group.key" class="overflow-hidden rounded-xl border border-border">
        <div class="sticky top-0 z-5 flex items-center justify-between bg-surface px-4 py-2 text-body font-medium">
          <span>{{ group.label }}</span>
          <span class="num flex items-center gap-3 text-xs text-text-secondary">
            <span>مدين <MoneyText :value="group.debit" plain /></span>
            <span>دائن <MoneyText :value="group.credit" plain /></span>
          </span>
        </div>
        <div>
          <div
            v-for="row in group.entries"
            :key="row.id"
            class="border-t border-border"
            :class="flatRows[activeIndex]?.id === row.id && 'bg-primary/5'"
          >
            <div class="flex cursor-pointer items-center gap-3 px-4 py-2.5 hover:bg-surface-hover" @click="openEntry(row)">
              <button type="button" class="rounded p-0.5 text-text-secondary hover:bg-surface-hover" @click.stop="toggleExpand(row.id)" aria-label="عرض الأسطر">
                <DirIcon :icon="dirIcon.open" class="size-4 transition-transform" :class="expanded.has(row.id) && '-rotate-90'" />
              </button>
              <span class="num w-24 shrink-0 font-medium">{{ row.number }}</span>
              <div class="min-w-0 flex-1">
                <p class="truncate">{{ row.description }}</p>
                <p v-if="row.sourceLabel" class="text-xs text-primary">
                  <RouterLink v-if="row.sourceLink" :to="row.sourceLink" class="hover:underline" @click.stop>
                    {{ row.sourceLabel }} <span class="num">{{ row.sourceRef?.number }}</span>
                  </RouterLink>
                </p>
              </div>
              <StatusBadge :tone="TYPE_TONE[row.type]" :label="TYPE_LABEL[row.type]" />
              <StatusBadge v-if="row.status === 'DRAFT'" tone="warning" label="مسودة" />
              <StatusBadge v-if="row.reversed" tone="danger" label="معكوس" />
              <span v-if="row.attachmentCount" class="flex items-center gap-0.5 text-xs text-text-secondary">
                <Paperclip class="size-3" /> {{ row.attachmentCount }}
              </span>
              <span class="w-32 shrink-0 text-end"><MoneyText :value="row.totalDebit" /></span>
              <span class="w-28 shrink-0 text-xs text-text-secondary">{{ row.createdByName }}</span>
            </div>
            <div v-if="expanded.has(row.id)" class="border-t border-border bg-surface/50 px-4 py-2">
              <table class="w-full text-xs">
                <thead class="text-text-secondary">
                  <tr>
                    <th class="py-1 text-start font-normal">الحساب</th>
                    <th class="py-1 text-start font-normal">العميل/المورد</th>
                    <th class="py-1 text-end font-normal">مدين</th>
                    <th class="py-1 text-end font-normal">دائن</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="l in row.lines" :key="l.id" class="border-t border-border/60">
                    <td class="py-1">{{ accountLabel(l.accountId) }}</td>
                    <td class="py-1 text-text-secondary">{{ l.partyId ?? '—' }}</td>
                    <td class="num py-1 text-end">{{ l.debit ? formatNumber(l.debit) : '' }}</td>
                    <td class="num py-1 text-end">{{ l.credit ? formatNumber(l.credit) : '' }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer totals -->
      <div class="flex items-center justify-end gap-6 rounded-xl border border-border bg-surface px-4 py-2.5 text-body font-medium">
        <span>الإجمالي</span>
        <span class="num">مدين <MoneyText :value="footerTotals.debit" plain /></span>
        <span class="num">دائن <MoneyText :value="footerTotals.credit" plain /></span>
      </div>
    </div>

    <AppModal v-model:open="saveViewModalOpen" title="حفظ العرض الحالي" size="sm">
      <AppInput v-model="newViewName" label="اسم العرض" required placeholder="مثال: قيود الصندوق هذا الشهر" />
      <template #footer>
        <AppButton @click="saveViewModalOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" @click="saveCurrentAsView">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
