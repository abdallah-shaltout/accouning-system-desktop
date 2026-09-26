<script setup lang="ts">
/**
 * v2 phase 7 (docs/v2/06-sales-and-pos.md §5 "Z-report is printed (thermal or A4)"). The A4 print is
 * an official document (letterhead, shift strip, sales by method, cash reconciliation, cashier /
 * supervisor signatures) rendered by the reports print pipeline — previewed, then printed or saved
 * as a native PDF — never a print of this screen.
 */
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
import ReportPrintDialog from '@/modules/reports/components/ReportPrintDialog.vue';
import { heading, kpis, money, note, row, table as printTable } from '@/modules/reports/print/build';
import { useOfficialPrint } from '@/modules/reports/print/useOfficialPrint';
import { getShift } from '../services/invoiceService';

const route = useRoute('pos-shift-report');
const id = String(route.params.id);
const { data, error, reload } = useAsync(() => getShift(id));
const shift = computed(() => data.value);

const { open: printOpen, doc: printDoc, show: showPrint } = useOfficialPrint();

function print() {
  const s = shift.value;
  if (!s) return;
  const variance = s.variance ?? 0;
  const cols = [
    { label: 'البند', width: 3 },
    { label: 'المبلغ', numeric: true, width: 1.4 },
  ];
  showPrint({
    title: 'تقرير إغلاق الوردية (Z)',
    subtitle: `وردية ${s.number}`,
    badge: 'تقرير Z',
    leadMeta: [
      { label: 'الوردية', value: s.number },
      { label: 'الكاشير', value: s.openedByName },
      { label: 'فُتحت', value: formatDateTime(s.openedAt) },
      { label: 'أُغلقت', value: s.closedAt ? formatDateTime(s.closedAt) : 'مفتوحة' },
    ],
    signatures: true,
    signatureTitles: ['الكاشير', 'المشرف / المدير'],
    blocks: [
      kpis([
        { label: 'إجمالي المبيعات', value: money(s.salesTotal) },
        { label: 'النقد المتوقع', value: money(s.expectedCash ?? 0) },
        { label: 'النقد المعدود', value: money(s.countedCash ?? 0) },
        { label: 'الفرق', value: money(variance), emphasis: true },
      ]),
      heading('المبيعات حسب طريقة الدفع'),
      printTable(cols, [...s.salesByMethod.map((m) => row([m.label, money(m.amount)])), row(['إجمالي المبيعات', money(s.salesTotal)], 'total')], 'لا توجد مبيعات في هذه الوردية'),
      heading('تسوية النقدية'),
      printTable(cols, [
        row(['الرصيد الافتتاحي', money(s.openingFloat)], 'opening'),
        row(['+ مبيعات نقدية', money(s.cashSales)]),
        row(['− مرتجعات نقدية', money(-s.cashRefunds)]),
        row(['+ إيداع (Pay in)', money(s.payIns)]),
        row(['− سحب (Pay out)', money(-s.payOuts)]),
        ...(s.bankDrops ? [row(['− إيداع بنكي', money(-s.bankDrops)])] : []),
        row(['النقد المتوقع في الدرج', money(s.expectedCash ?? 0)], 'subtotal'),
        row(['النقد المعدود فعلياً', money(s.countedCash ?? 0)]),
        row([variance < 0 ? 'الفرق (عجز)' : variance > 0 ? 'الفرق (زيادة)' : 'الفرق', money(variance)], 'grand'),
      ]),
      ...(Math.abs(variance) >= 0.01 ? [note(variance < 0 ? `يوجد عجز في الصندوق بقيمة ${money(Math.abs(variance))}` : `توجد زيادة في الصندوق بقيمة ${money(variance)}`, 'warn')] : [note('الصندوق مطابق — لا يوجد فرق', 'ok')]),
    ],
  });
}
</script>

<template>
  <div>
    <ErrorState v-if="error" :message="error" @retry="reload" />
    <template v-else>
      <PageHeader :title="shift ? `تقرير Z — وردية ${shift.number}` : '…'" :back="{ name: 'pos-shifts' }">
        <template #actions><AppButton variant="primary" :icon="Printer" :disabled="!shift" data-testid="shift-print" @click="print">طباعة</AppButton></template>
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

    <ReportPrintDialog v-model:open="printOpen" :doc="printDoc" :file-name="`تقرير Z ${shift?.number ?? ''}`" />
  </div>
</template>
