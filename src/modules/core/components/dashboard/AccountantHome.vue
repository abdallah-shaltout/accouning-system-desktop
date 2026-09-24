<script setup lang="ts">
import { computed } from 'vue';
import { Banknote, FileText, Landmark, ReceiptText } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import KpiCard from '@/modules/core/components/dashboard/KpiCard.vue';
import RiyalIcon from '@/modules/core/components/ui/RiyalIcon.vue';
import NeedsAttentionPanel from '@/modules/core/components/insights/NeedsAttentionPanel.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateLong, formatMoney } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { db } from '@/mocks/db';
import { getDashboardSummary } from '@/modules/core/services/dashboardService';

/**
 * v2 (docs/v2/01-personas.md §3, docs/v2/11 Part B "role homes" table): "Insight cards + cash &
 * banks (per account) + receivables/payables aging bars + VAT period box + drafts count" — a home
 * focused on what needs posting/approving, built entirely on the insight engine (D1) plus a small
 * cash/drafts snapshot. Completes this phase's accountant-role row.
 */
const auth = useAuthStore();
const summary = useAsync(getDashboardSummary);

const draftCount = computed(() => db.journalDrafts.length);
const greeting = computed(() => (new Date().getHours() < 12 ? 'صباح الخير' : 'مساء الخير'));
</script>

<template>
  <div>
    <div class="mb-5 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 class="text-lg font-semibold tracking-tight">{{ greeting }}، {{ auth.user?.name.split(' ')[0] }}</h1>
        <p class="mt-0.5 text-body text-text-secondary">{{ formatDateLong(new Date().toISOString()) }}</p>
      </div>
      <div class="flex gap-2">
        <AppButton :icon="FileText" to="/accounting/journal/new">قيد جديد</AppButton>
        <AppButton variant="primary" :icon="ReceiptText" to="/expenses">مصروف جديد</AppButton>
      </div>
    </div>

    <div class="mb-4">
      <NeedsAttentionPanel />
    </div>

    <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      <KpiCard label="الصندوق" :icon="Banknote" :loading="summary.loading.value" to="/reports/ledger?account=acc-1110">
        <span dir="ltr">{{ formatMoney(summary.data.value?.cashOnHand) }}</span> <RiyalIcon class="text-[0.7em] text-text-secondary" />
      </KpiCard>
      <KpiCard label="البنك" :icon="Landmark" :loading="summary.loading.value" to="/reports/ledger?account=acc-1120">
        <span dir="ltr">{{ formatMoney(summary.data.value?.bankBalance) }}</span> <RiyalIcon class="text-[0.7em] text-text-secondary" />
      </KpiCard>
      <KpiCard label="فواتير غير مسددة" :icon="ReceiptText" :loading="summary.loading.value" :to="{ path: '/invoices', query: { payment: 'open' } }">
        {{ summary.data.value?.unpaidInvoiceCount }}
        <template #hint>مستحق: <span class="num">{{ formatMoney(summary.data.value?.unpaidInvoiceTotal) }}</span></template>
      </KpiCard>
      <KpiCard label="مسودات بحاجة لترحيل" :icon="FileText" :loading="summary.loading.value" :tone="draftCount > 0 ? 'warning' : undefined" to="/accounting/journal">
        {{ draftCount }}
      </KpiCard>
    </div>

    <div class="mt-4 grid gap-4 lg:grid-cols-2">
      <AppCard title="تسوية ضريبة القيمة المضافة" padding="none">
        <RouterLink to="/accounting/vat-settlement" class="flex items-center justify-between gap-3 px-4 py-3 hover:bg-surface-hover">
          <span class="text-body">عرض التسوية والموعد النهائي</span>
        </RouterLink>
      </AppCard>
      <AppCard title="الذمم والتقارير" padding="none">
        <div class="flex flex-wrap gap-2 p-4">
          <AppButton size="sm" to="/customers">أرصدة العملاء</AppButton>
          <AppButton size="sm" to="/suppliers">أرصدة الموردين</AppButton>
          <AppButton size="sm" to="/reports">كل التقارير</AppButton>
        </div>
      </AppCard>
    </div>
  </div>
</template>
