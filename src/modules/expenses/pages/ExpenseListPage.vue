<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { CalendarClock, Plus, Receipt } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDate, startOfMonthKey } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getDueRecurringExpenses, getExpenseCategories, getExpenses, postDueRecurringExpense, type ExpenseRow } from '../services/expenseService';
import type { ExpenseCategory, RecurringExpense } from '../types';

const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
const categoryId = ref('');
const search = ref('');
const from = ref('');
const to = ref('');
const categories = ref<ExpenseCategory[]>([]);
const due = ref<RecurringExpense[]>([]);
const posting = ref<string | null>(null);

const { data, loading, error, reload } = useAsync(() => getExpenses({ categoryId: categoryId.value || undefined, from: from.value || undefined, to: to.value || undefined, search: search.value || undefined }));
watch([categoryId, from, to, search], reload);

onMounted(async () => {
  categories.value = await getExpenseCategories();
  due.value = await getDueRecurringExpenses();
});

async function postDue(id: string) {
  posting.value = id;
  try {
    const expense = await postDueRecurringExpense(id);
    toast.success('تم تسجيل المصروف المتكرر', expense.number);
    due.value = await getDueRecurringExpenses();
    await reload();
  } catch (err) {
    toast.error(err);
  } finally {
    posting.value = null;
  }
}

const categoryOptions = computed(() => [{ value: '', label: 'كل التصنيفات' }, ...categories.value.map((c) => ({ value: c.id, label: c.name }))]);

// Month total by category (mini bar chart) — current month's posted expenses.
const monthTotals = computed(() => {
  const monthStart = startOfMonthKey();
  const byCategory = new Map<string, number>();
  for (const e of data.value ?? []) {
    if (e.date.slice(0, 10) < monthStart) continue;
    byCategory.set(e.categoryName, round((byCategory.get(e.categoryName) ?? 0) + e.amount));
  }
  const entries = [...byCategory.entries()].sort((a, b) => b[1] - a[1]);
  const max = Math.max(1, ...entries.map(([, v]) => v));
  return entries.map(([name, total]) => ({ name, total, pct: Math.round((total / max) * 100) }));
});
function round(n: number) {
  return Math.round(n * 100) / 100;
}

const columns: Column<ExpenseRow>[] = [
  { key: 'number', label: 'الرقم', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'categoryName', label: 'التصنيف', sortable: true },
  { key: 'description', label: 'الوصف' },
  { key: 'amount', label: 'المبلغ', numeric: true, sortable: true },
];
</script>

<template>
  <div>
    <PageHeader title="المصروفات" subtitle="تسجيل المصروفات التشغيلية وربطها بحسابات دليل الحسابات">
      <template v-if="auth.can('purchases', 'write') || auth.can('accounting', 'write')" #actions>
        <AppButton variant="primary" :icon="Plus" to="/expenses/new">مصروف جديد</AppButton>
      </template>
    </PageHeader>

    <AppCard v-if="due.length" title="مصروفات متكررة مستحقة" padding="sm" class="mb-4">
      <ul class="divide-y divide-border text-body">
        <li v-for="r in due" :key="r.id" class="flex items-center justify-between py-2">
          <div class="flex items-center gap-2">
            <CalendarClock class="size-4 text-warning" />
            <span>{{ r.name }}</span>
            <span class="num text-xs text-text-secondary">مستحق {{ formatDate(r.nextDate) }}</span>
          </div>
          <AppButton size="sm" variant="primary" :loading="posting === r.id" @click="postDue(r.id)">تسجيل الآن</AppButton>
        </li>
      </ul>
    </AppCard>

    <AppCard v-if="monthTotals.length" title="إجمالي الشهر حسب التصنيف" padding="sm" class="mb-4">
      <div class="space-y-2">
        <div v-for="m in monthTotals" :key="m.name" class="flex items-center gap-3">
          <span class="w-28 shrink-0 truncate text-xs text-text-secondary">{{ m.name }}</span>
          <div class="h-2 flex-1 overflow-hidden rounded-full bg-surface">
            <div class="h-full rounded-full bg-primary" :style="{ width: `${m.pct}%` }" />
          </div>
          <MoneyText :value="m.total" plain class="w-20 shrink-0 text-end text-xs" />
        </div>
      </div>
    </AppCard>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <div class="flex flex-wrap items-center gap-2">
        <AppSelect v-model="categoryId" :options="categoryOptions" />
        <DateRangeFilter v-model:from="from" v-model:to="to" />
      </div>
      <SearchInput v-model="search" placeholder="الرقم أو الوصف" />
    </div>

    <DataTable
      :columns="columns"
      :rows="data ?? []"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="Receipt"
      empty-title="لا توجد مصروفات"
      @retry="reload"
      @row-click="(r) => router.push(`/expenses/${r.id}`)"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDate(row.date) }}</span></template>
      <template #cell-amount="{ row }"><MoneyText :value="row.amount" /></template>
    </DataTable>
  </div>
</template>
