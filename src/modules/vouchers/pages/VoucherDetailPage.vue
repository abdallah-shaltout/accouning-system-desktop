<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { BookOpen, Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime } from '@/modules/core/helpers/format';
import { db } from '@/mocks';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getVoucher } from '../services/voucherService';
import type { VoucherKind } from '../types';

const route = useRoute();
const auth = useAuthStore();
const id = String(route.params.id);
const { data, error, reload } = useAsync(() => getVoucher(id));

const KIND_LABEL: Record<VoucherKind, string> = { RECEIPT: 'سند قبض عام', PAYMENT: 'سند صرف عام', TRANSFER: 'تحويل بين الحسابات', OWNER: 'سند مالك' };

const journalEntries = computed(() => db.journalEntries.filter((e) => e.sourceRef?.kind === 'voucher' && e.sourceRef.id === id).map((e) => ({ id: e.id, number: e.number, description: e.description })));
</script>

<template>
  <div>
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <template v-else>
      <PageHeader :title="data ? `${KIND_LABEL[data.kind]} ${data.number}` : '…'" back="/vouchers">
        <template v-if="data" #actions>
          <AppButton :icon="Printer" :to="`/print/vouchers/${id}`">طباعة</AppButton>
        </template>
      </PageHeader>
      <div class="grid items-start gap-5 xl:grid-cols-[1fr_320px]">
        <AppCard padding="sm">
          <SkeletonBlock v-if="!data" :lines="6" />
          <dl v-else class="space-y-1.5 text-body">
            <div class="flex justify-between"><dt class="text-text-secondary">التاريخ</dt><dd class="num">{{ formatDateTime(data.date) }}</dd></div>
            <div class="flex justify-between"><dt class="text-text-secondary">الوصف</dt><dd>{{ data.description }}</dd></div>
            <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>المبلغ</dt><dd><MoneyText :value="data.amount" /></dd></div>
          </dl>
          <p v-if="data?.note" class="mt-3 border-t border-border pt-2 text-xs text-text-secondary">{{ data.note }}</p>
        </AppCard>
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
