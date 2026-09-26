<script setup lang="ts">
// Generic voucher print (docs/v2/09-purchases-payments-expenses.md §5 "Each voucher prints as a
// PDF"). pdfService only has the invoice template until Phase 11b (docs/v2/15-action-plan.md), so
// this minimal browser print route is the sanctioned fallback for every voucher kind.
import { onBeforeUnmount } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useHotkeys } from '@/modules/core/controllers/useHotkeys';
import { dirIcon } from '@/modules/core/helpers/dirIcon';
import { formatDate, formatMoney } from '@/modules/core/helpers/format';
import { tafqit } from '@/modules/core/helpers/tafqit';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { getVoucher } from '../services/voucherService';
import type { VoucherKind } from '../types';

const route = useRoute('voucher-print');
const router = useRouter();
const settings = useSettingsStore();
const id = String(route.params.id);

const { data, error, reload } = useAsync(() => getVoucher(id));

const KIND_LABEL: Record<VoucherKind, string> = { RECEIPT: 'سند قبض عام', PAYMENT: 'سند صرف عام', TRANSFER: 'سند تحويل', OWNER: 'سند مالك' };

const pageStyle = document.createElement('style');
document.head.appendChild(pageStyle);
pageStyle.textContent = '@page { size: A5; margin: 10mm; }';
onBeforeUnmount(() => pageStyle.remove());

function print() {
  window.print();
}
function close() {
  router.push({ name: 'voucher-detail', params: { id } });
}
useHotkeys({ 'ctrl+p': { id: 'print.document', label: 'طباعة المستند', group: 'الطباعة', handler: print }, Escape: close });
</script>

<template>
  <div class="min-h-screen bg-surface">
    <div class="no-print sticky top-0 z-10 flex h-12 items-center justify-between gap-3 border-b border-border bg-background px-4">
      <div class="flex items-center gap-2">
        <AppButton size="sm" variant="ghost" :icon="dirIcon.back" icon-rtl-flip @click="close">رجوع</AppButton>
        <span class="text-body font-medium">{{ data ? KIND_LABEL[data.kind] : 'سند' }}</span>
      </div>
      <AppButton size="sm" variant="primary" :icon="Printer" kbd="Ctrl+P" :disabled="!data" @click="print">طباعة</AppButton>
    </div>
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <div v-else class="flex justify-center p-8 print:p-0">
      <div v-if="!data" class="w-[148mm] rounded-lg bg-white p-8"><SkeletonBlock :lines="10" /></div>
      <div v-else class="print-root w-[148mm] bg-white p-[10mm] text-black shadow-lg ring-1 ring-black/5 print:shadow-none print:ring-0">
        <header class="mb-4 flex items-start justify-between border-b border-black/10 pb-3">
          <h1 class="text-sm font-bold">{{ settings.settings?.storeName }}</h1>
          <div class="text-end">
            <h2 class="text-base font-bold">{{ KIND_LABEL[data.kind] }}</h2>
            <p class="num text-xs text-black/60">{{ data.number }} — {{ formatDate(data.date) }}</p>
          </div>
        </header>
        <p class="mb-3 text-sm">{{ data.description }}</p>
        <div class="mb-4 rounded border border-black/10 p-3 text-center">
          <p class="num text-2xl font-bold">{{ formatMoney(data.amount) }}</p>
          <p class="mt-1 text-xs text-black/60">{{ tafqit(data.amount) }}</p>
        </div>
        <p v-if="data.note" class="mb-6 text-xs text-black/60">{{ data.note }}</p>
        <div class="mt-10 grid grid-cols-2 gap-8 text-xs">
          <div class="border-t border-black/30 pt-1 text-center">المستلم/المُعد</div>
          <div class="border-t border-black/30 pt-1 text-center">المعتمد</div>
        </div>
      </div>
    </div>
  </div>
</template>
