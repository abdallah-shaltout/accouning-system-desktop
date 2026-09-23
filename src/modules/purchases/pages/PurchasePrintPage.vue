<script setup lang="ts">
// v2 phase 8 "إرسال للمورد" (docs/v2/09-purchases-payments-expenses.md §1). pdfService only has the
// invoice template (Phase 11a scope) — the other 9 document kinds, including the purchase order,
// land in Phase 11b. Until then this generic browser print route is the sanctioned fallback (same
// approach InvoicePrintPage.vue uses for the v1 print path).
import { computed, onBeforeUnmount } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ArrowRight, Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useHotkeys } from '@/modules/core/controllers/useHotkeys';
import { formatDate, formatNumber } from '@/modules/core/helpers/format';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { getPurchaseOrder } from '../services/purchaseService';

const route = useRoute();
const router = useRouter();
const settings = useSettingsStore();
const id = String(route.params.id);

const { data, error, reload } = useAsync(() => getPurchaseOrder(id));

const pageStyle = document.createElement('style');
document.head.appendChild(pageStyle);
pageStyle.textContent = '@page { size: A4; margin: 12mm; }';
onBeforeUnmount(() => pageStyle.remove());

function print() {
  window.print();
}
function close() {
  router.push(`/purchases/${id}`);
}
useHotkeys({ 'ctrl+p': print, Escape: close });

const lineTotal = (l: { qty: number; costPrice: number }) => l.qty * l.costPrice;
const total = computed(() => (data.value?.lines ?? []).reduce((a, l) => a + lineTotal(l), 0));
</script>

<template>
  <div class="min-h-screen bg-surface">
    <div class="no-print sticky top-0 z-10 flex h-12 items-center justify-between gap-3 border-b border-border bg-background px-4">
      <div class="flex items-center gap-2">
        <AppButton size="sm" variant="ghost" :icon="ArrowRight" @click="close">رجوع</AppButton>
        <span class="text-body font-medium">أمر شراء</span>
        <span v-if="data" class="num text-body text-text-secondary">{{ data.number }}</span>
      </div>
      <AppButton size="sm" variant="primary" :icon="Printer" kbd="Ctrl+P" :disabled="!data" @click="print">طباعة</AppButton>
    </div>

    <ErrorState v-if="error" :message="error" @retry="reload" />
    <div v-else class="flex justify-center p-8 print:p-0">
      <div v-if="!data" class="w-[186mm] rounded-lg bg-white p-10"><SkeletonBlock :lines="14" /></div>
      <div v-else class="print-root w-[186mm] bg-white p-[12mm] text-black shadow-lg ring-1 ring-black/5 print:shadow-none print:ring-0">
        <header class="mb-6 flex items-start justify-between border-b border-black/10 pb-4">
          <div>
            <h1 class="text-lg font-bold">{{ settings.settings?.storeName }}</h1>
            <p class="text-xs text-black/60">{{ settings.settings?.address }}</p>
            <p class="text-xs text-black/60">{{ settings.settings?.phone }}</p>
          </div>
          <div class="text-end">
            <h2 class="text-base font-bold">أمر شراء</h2>
            <p class="num text-xs text-black/60">{{ data.number }}</p>
            <p class="num text-xs text-black/60">{{ formatDate(data.date) }}</p>
          </div>
        </header>
        <section class="mb-4 text-sm">
          <p class="font-medium">المورد: {{ data.supplierName }}</p>
          <p v-if="data.supplier?.phone" class="num text-xs text-black/60">{{ data.supplier.phone }}</p>
        </section>
        <table class="w-full border-collapse text-sm">
          <thead>
            <tr class="border-b border-black/20 text-xs">
              <th class="p-2 text-start">الصنف</th>
              <th class="p-2 text-start">الكمية</th>
              <th class="p-2 text-start">سعر التكلفة</th>
              <th class="p-2 text-start">الإجمالي</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="l in data.lines" :key="l.productId" class="border-b border-black/10">
              <td class="p-2">{{ data.products[l.productId]?.name }}</td>
              <td class="num p-2">{{ formatNumber(l.qty) }}</td>
              <td class="num p-2"><MoneyText :value="l.costPrice" plain /></td>
              <td class="num p-2"><MoneyText :value="lineTotal(l)" plain /></td>
            </tr>
          </tbody>
        </table>
        <div class="mt-4 flex justify-end">
          <dl class="w-56 space-y-1 text-sm">
            <div class="flex justify-between"><dt>الإجمالي</dt><dd><MoneyText :value="total" plain /></dd></div>
          </dl>
        </div>
        <p v-if="data.note" class="mt-6 border-t border-black/10 pt-2 text-xs text-black/60">{{ data.note }}</p>
      </div>
    </div>
  </div>
</template>
