<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { BookOpen } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime } from '@/modules/core/helpers/format';
import { db } from '@/mocks';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getExpense } from '../services/expenseService';

const route = useRoute('expense-detail');
const auth = useAuthStore();
const id = String(route.params.id);
const { data, error, reload } = useAsync(() => getExpense(id));

const journalEntries = computed(() => db.journalEntries.filter((e) => e.sourceRef?.kind === 'expense' && e.sourceRef.id === id).map((e) => ({ id: e.id, number: e.number, description: e.description })));
</script>

<template>
  <div>
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <template v-else>
      <PageHeader :title="data ? `مصروف ${data.number}` : '…'" :subtitle="data?.categoryName" back="/expenses" />
      <div class="grid items-start gap-5 xl:grid-cols-[1fr_320px]">
        <div class="space-y-5">
          <AppCard padding="sm">
            <SkeletonBlock v-if="!data" :lines="6" />
            <dl v-else class="space-y-1.5 text-body">
              <div class="flex justify-between"><dt class="text-text-secondary">التاريخ</dt><dd class="num">{{ formatDateTime(data.date) }}</dd></div>
              <div class="flex justify-between"><dt class="text-text-secondary">صافي</dt><dd><MoneyText :value="data.netAmount" /></dd></div>
              <div v-if="data.taxAmount" class="flex justify-between"><dt class="text-text-secondary">الضريبة</dt><dd><MoneyText :value="data.taxAmount" /></dd></div>
              <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>الإجمالي</dt><dd><MoneyText :value="data.amount" /></dd></div>
              <div v-if="data.supplierInvoiceNo" class="flex justify-between"><dt class="text-text-secondary">رقم فاتورة المورد</dt><dd class="num">{{ data.supplierInvoiceNo }}</dd></div>
            </dl>
            <p v-if="data?.description" class="mt-3 border-t border-border pt-2 text-xs text-text-secondary">{{ data.description }}</p>
          </AppCard>
        </div>
        <div class="space-y-4">
          <AppCard v-if="journalEntries.length && auth.can('accounting')" title="القيود المحاسبية" padding="none">
            <ul class="divide-y divide-border text-body">
              <li v-for="e in journalEntries" :key="e.id">
                <RouterLink :to="`/accounting/journal/${e.id}`" class="flex items-center gap-2 px-4 py-2 hover:bg-surface-hover">
                  <BookOpen class="size-3.5 shrink-0 text-text-secondary" />
                  <span class="num text-primary">{{ e.number }}</span>
                  <span class="truncate text-xs text-text-secondary">{{ e.description }}</span>
                </RouterLink>
              </li>
            </ul>
          </AppCard>
        </div>
      </div>
    </template>
  </div>
</template>
