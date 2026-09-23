<script setup lang="ts">
/**
 * Day book (دفتر اليومية) print preview (docs/v2/11-journal-dashboard-insights.md A1). `pdfService`
 * only builds an `invoice` payload so far (Phase 11a scope — see its doc comment); a real Typst
 * day-book template is Phase 11b/12 territory. This is the same browser-print fallback
 * `InvoicePrintPage` already uses for non-Tauri, applied here directly rather than through
 * `pdfService` — a documented shortcut, not a silent gap (see the phase 2 report).
 */
import { computed, onBeforeUnmount, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ArrowRight, Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useHotkeys } from '@/modules/core/controllers/useHotkeys';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { formatDate, formatDateTime, todayKey } from '@/modules/core/helpers/format';
import { getAccounts, getJournalEntries, type AccountWithBalance } from '../services/accountingService';

const route = useRoute();
const router = useRouter();
const settings = useSettingsStore();

const from = computed(() => (typeof route.query.from === 'string' ? route.query.from : todayKey()));
const to = computed(() => (typeof route.query.to === 'string' ? route.query.to : todayKey()));

const { data: entries, error, reload } = useAsync(() => getJournalEntries({ from: from.value, to: to.value, status: 'POSTED' }));
const { data: accounts } = useAsync(getAccounts);
watch([from, to], reload);

function accountLabel(id: string) {
  const a = (accounts.value as AccountWithBalance[] | undefined)?.find((x) => x.id === id);
  return a ? `${a.code} — ${a.name}` : id;
}

const totals = computed(() => ({
  debit: (entries.value ?? []).reduce((a, e) => a + e.totalDebit, 0),
  credit: (entries.value ?? []).reduce((a, e) => a + e.totalCredit, 0),
}));

const pageStyle = document.createElement('style');
document.head.appendChild(pageStyle);
pageStyle.textContent = '@page { size: A4; margin: 12mm; }';
onBeforeUnmount(() => pageStyle.remove());

function print() {
  window.print();
}
function close() {
  router.push('/accounting/journal');
}
useHotkeys({ 'ctrl+p': print, Escape: close });
</script>

<template>
  <div class="min-h-screen bg-surface">
    <div class="no-print sticky top-0 z-10 flex h-12 items-center justify-between gap-3 border-b border-border bg-background px-4">
      <div class="flex items-center gap-2">
        <AppButton size="sm" variant="ghost" :icon="ArrowRight" @click="close">رجوع</AppButton>
        <span class="text-body font-medium">دفتر اليومية</span>
      </div>
      <AppButton size="sm" variant="primary" :icon="Printer" kbd="Ctrl+P" @click="print">طباعة</AppButton>
    </div>

    <ErrorState v-if="error" :message="error" @retry="reload" />
    <div v-else class="flex justify-center p-8 print:p-0">
      <div class="print-root w-[186mm] bg-white p-[12mm] text-black shadow-lg ring-1 ring-black/5 print:shadow-none print:ring-0">
        <div v-if="!entries"><SkeletonBlock :lines="14" /></div>
        <template v-else>
          <header class="mb-6 text-center">
            <h1 class="text-xl font-bold">{{ settings.settings?.storeName ?? '—' }}</h1>
            <p class="mt-1 text-lg">دفتر اليومية</p>
            <p class="num mt-1 text-sm text-gray-600">{{ formatDate(from) }} — {{ formatDate(to) }}</p>
          </header>
          <table class="w-full border-collapse text-sm">
            <thead>
              <tr class="border-b-2 border-black">
                <th class="p-1.5 text-start">رقم القيد</th>
                <th class="p-1.5 text-start">التاريخ</th>
                <th class="p-1.5 text-start">البيان</th>
                <th class="p-1.5 text-start">الحساب</th>
                <th class="p-1.5 text-end">مدين</th>
                <th class="p-1.5 text-end">دائن</th>
              </tr>
            </thead>
            <tbody>
              <template v-for="e in entries" :key="e.id">
                <tr v-for="(l, i) in e.lines" :key="l.id" class="border-b border-gray-300">
                  <td class="num p-1.5">{{ i === 0 ? e.number : '' }}</td>
                  <td class="num p-1.5">{{ i === 0 ? formatDateTime(e.date) : '' }}</td>
                  <td class="p-1.5">{{ i === 0 ? e.description : '' }}</td>
                  <td class="p-1.5">{{ accountLabel(l.accountId) }}</td>
                  <td class="num p-1.5 text-end">{{ l.debit ? l.debit.toFixed(2) : '' }}</td>
                  <td class="num p-1.5 text-end">{{ l.credit ? l.credit.toFixed(2) : '' }}</td>
                </tr>
              </template>
            </tbody>
            <tfoot>
              <tr class="border-t-2 border-black font-bold">
                <td class="p-1.5" colspan="4">الإجمالي</td>
                <td class="p-1.5 text-end"><MoneyText :value="totals.debit" plain /></td>
                <td class="p-1.5 text-end"><MoneyText :value="totals.credit" plain /></td>
              </tr>
            </tfoot>
          </table>
        </template>
      </div>
    </div>
  </div>
</template>
