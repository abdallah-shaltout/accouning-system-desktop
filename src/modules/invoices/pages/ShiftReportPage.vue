<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §5 "Z-report is printed (thermal or A4)"). Reuses the browser print flow like every other document until 11b adds a dedicated Typst template. */
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime } from '@/modules/core/helpers/format';
import { getShift } from '../services/invoiceService';

const route = useRoute();
const id = String(route.params.id);
const { data, error, reload } = useAsync(() => getShift(id));
const shift = computed(() => data.value);

function print() {
  window.print();
}
</script>

<template>
  <div>
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <template v-else>
      <PageHeader :title="shift ? `تقرير Z — وردية ${shift.number}` : '…'" back="/pos/shifts">
        <template #actions><AppButton variant="primary" :icon="Printer" @click="print">طباعة</AppButton></template>
      </PageHeader>

      <AppCard v-if="!shift" padding="sm"><SkeletonBlock :lines="8" /></AppCard>
      <div v-else class="mx-auto max-w-md space-y-3 rounded-lg border border-border bg-surface p-5 text-body" dir="rtl">
        <p class="text-center text-lg font-semibold">تقرير إغلاق الوردية (Z)</p>
        <dl class="space-y-1">
          <div class="flex justify-between"><dt class="text-text-secondary">الوردية</dt><dd class="num">{{ shift.number }}</dd></div>
          <div class="flex justify-between"><dt class="text-text-secondary">الكاشير</dt><dd>{{ shift.openedByName }}</dd></div>
          <div class="flex justify-between"><dt class="text-text-secondary">فُتحت</dt><dd class="num">{{ formatDateTime(shift.openedAt) }}</dd></div>
          <div class="flex justify-between"><dt class="text-text-secondary">أُغلقت</dt><dd class="num">{{ shift.closedAt ? formatDateTime(shift.closedAt) : '—' }}</dd></div>
        </dl>
        <div class="border-t border-dashed border-border" />
        <dl class="space-y-1">
          <div v-for="s in shift.salesByMethod" :key="s.label" class="flex justify-between"><dt class="text-text-secondary">{{ s.label }}</dt><dd><MoneyText :value="s.amount" plain /></dd></div>
          <div class="flex justify-between font-medium"><dt>إجمالي المبيعات</dt><dd><MoneyText :value="shift.salesTotal" plain /></dd></div>
        </dl>
        <div class="border-t border-dashed border-border" />
        <dl class="space-y-1">
          <div class="flex justify-between"><dt class="text-text-secondary">الرصيد الافتتاحي</dt><dd><MoneyText :value="shift.openingFloat" plain /></dd></div>
          <div class="flex justify-between"><dt class="text-text-secondary">مبيعات نقدية</dt><dd><MoneyText :value="shift.cashSales" plain /></dd></div>
          <div class="flex justify-between"><dt class="text-text-secondary">مرتجعات نقدية</dt><dd>−<MoneyText :value="shift.cashRefunds" plain /></dd></div>
          <div class="flex justify-between"><dt class="text-text-secondary">إيداع (Pay in)</dt><dd><MoneyText :value="shift.payIns" plain /></dd></div>
          <div class="flex justify-between"><dt class="text-text-secondary">سحب (Pay out)</dt><dd>−<MoneyText :value="shift.payOuts" plain /></dd></div>
          <div class="flex justify-between font-medium border-t border-border pt-1"><dt>النقد المتوقع</dt><dd><MoneyText :value="shift.expectedCash ?? 0" plain /></dd></div>
          <div class="flex justify-between"><dt class="text-text-secondary">النقد المعدود</dt><dd><MoneyText :value="shift.countedCash ?? 0" plain /></dd></div>
          <div class="flex justify-between font-semibold" :class="(shift.variance ?? 0) < 0 ? 'text-danger' : (shift.variance ?? 0) > 0 ? 'text-primary' : 'text-success'">
            <dt>الفرق</dt><dd><MoneyText :value="shift.variance ?? 0" signed plain /></dd>
          </div>
        </dl>
      </div>
    </template>
  </div>
</template>
