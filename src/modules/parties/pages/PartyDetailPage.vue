<script setup lang="ts">
/**
 * Party page (docs/v2/08-customers-and-suppliers.md §3): header card (balance, credit-limit bar,
 * actions) + tabs — overview / documents / payments / statement / aging / attachments / history.
 * The attachments tab (Phase 0 demo usage) is kept as-is; the rest are new for Phase 4.
 */
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import {
  ClipboardList,
  Clock,
  FileText,
  HandCoins,
  History as HistoryIcon,
  Mail,
  MapPin,
  MessageCircle,
  Pencil,
  Phone,
  ReceiptText,
  UserRound,
} from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import DetailPage, { type DetailTab } from '@/modules/core/components/layouts/DetailPage.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate, formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { INVOICE_STATUS, PHONE_LABEL, PURCHASE_STATUS } from '@/modules/core/helpers/labels';
import { getInvoices } from '@/modules/invoices/services/invoiceService';
import { getOpenDocuments } from '@/modules/payments/services/paymentService';
import type { AllocationStatus, OpenDocument } from '@/modules/payments/types';
import { getPayments } from '@/modules/payments/services/paymentService';
import { getPurchaseOrders } from '@/modules/purchases/services/purchaseService';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import type { AppRoute } from '@/modules/core/types/route';
import InsightHints from '@/modules/core/components/insights/InsightHints.vue';
import { partyRoute } from '../helpers/partyRoutes';
import { getPartyAging, getPartyHistory } from '../services/partyService';

/** v2 phase 10 (docs/v2/11 D1 "inline hints"): both rules are already one-instance-per-customer. */
const CUSTOMER_INSIGHT_RULES = ['overdue-customers', 'credit-limit'];
import { getCustomer, getCustomerStatement, getSupplier, getSupplierStatement } from '../services/partyService';
import type { AgingBucket, Customer, PartyStatementRow, Supplier } from '../types';

const props = defineProps<{ kind: 'customer' | 'supplier' }>();

const route = useRoute<'customer' | 'supplier'>();
const router = useRouter();
const auth = useAuthStore();
const id = String(route.params.id);
const isCustomer = computed(() => props.kind === 'customer');
const base = computed(() => partyRoute(props.kind, 'list'));

const party = useAsync<Customer | Supplier>(() => (isCustomer.value ? getCustomer(id) : getSupplier(id)));
const statement = useAsync(() => (isCustomer.value ? getCustomerStatement(id) : getSupplierStatement(id)));
const open = useAsync(() => getOpenDocuments(props.kind, id));
const documents = useAsync<any[]>(() => (isCustomer.value ? getInvoices({ customerId: id }) : getPurchaseOrders({ supplierId: id })));
const payments = useAsync(() => getPayments({ targetId: id }));
const aging = useAsync<AgingBucket[]>(() => getPartyAging(props.kind, id));
const history = useAsync(() => getPartyHistory(id));

const ownerRef = computed(() => `${props.kind}:${id}`);
const p = computed(() => party.data.value);

function withCount(label: string, count?: number): string {
  return count ? `${label} (${formatNumber(count)})` : label;
}

const tabs = computed<DetailTab[]>(() => [
  { key: 'overview', label: 'نظرة عامة' },
  { key: 'documents', label: withCount('المستندات', documents.data.value?.length) },
  { key: 'payments', label: withCount('المدفوعات', payments.data.value?.length) },
  { key: 'statement', label: withCount('كشف الحساب', statement.data.value?.length) },
  { key: 'aging', label: 'الأعمار' },
  { key: 'attachments', label: 'المرفقات' },
  { key: 'history', label: withCount('السجل', history.data.value?.length) },
]);

const ALLOCATION_LABEL: Record<AllocationStatus, string> = { full: 'مخصص بالكامل', partial: 'مخصص جزئياً', unallocated: 'غير مخصص' };
const ALLOCATION_TONE: Record<AllocationStatus, 'success' | 'warning' | 'neutral'> = { full: 'success', partial: 'warning', unallocated: 'neutral' };

function docStatus(status: string) {
  return isCustomer.value ? INVOICE_STATUS[status as keyof typeof INVOICE_STATUS] : PURCHASE_STATUS[status as keyof typeof PURCHASE_STATUS];
}

const totals = computed(() => {
  const rows = statement.data.value ?? [];
  const docs = rows.filter((r) => r.kind === (isCustomer.value ? 'invoice' : 'purchaseOrder') && (isCustomer.value ? r.debit > 0 : r.credit > 0));
  return {
    documents: docs.length,
    volume: docs.reduce((a, r) => a + r.debit + r.credit, 0),
    lastDate: rows.at(-1)?.date,
  };
});

const overdueAmount = computed(() => (aging.data.value ?? []).filter((b) => b.key !== 'current').reduce((a, b) => a + b.total, 0));
const creditLimit = computed(() => (isCustomer.value ? (p.value as Customer | undefined)?.creditLimit ?? 0 : 0));
const creditPct = computed(() => (creditLimit.value > 0 ? Math.min(100, Math.round(((p.value?.balance ?? 0) / creditLimit.value) * 100)) : 0));
const overLimit = computed(() => creditLimit.value > 0 && (p.value?.balance ?? 0) > creditLimit.value);

function docLink(row: PartyStatementRow): AppRoute {
  if (row.kind === 'invoice' || row.kind === 'refund') return { name: 'invoice', params: { id: row.refId } };
  if (row.kind === 'purchaseOrder' || row.kind === 'purchaseReturn') return { name: 'purchase', params: { id: row.refId } };
  // v2 phase 5 (docs/v2/05-onboarding.md §4): an "opening" row has no document page of its own —
  // the journal entry itself is the record.
  if (row.kind === 'opening') return { name: 'journal-entry', params: { id: row.refId } };
  return { name: 'payment-detail', params: { id: row.refId } };
}

const statementColumns: Column<PartyStatementRow>[] = [
  { key: 'date', label: 'التاريخ' },
  { key: 'number', label: 'المستند' },
  { key: 'description', label: 'البيان' },
  { key: 'debit', label: 'مدين', numeric: true },
  { key: 'credit', label: 'دائن', numeric: true },
  { key: 'balance', label: 'الرصيد', numeric: true },
];

const openColumns: Column<OpenDocument>[] = [
  { key: 'number', label: 'المستند' },
  { key: 'date', label: 'التاريخ' },
  { key: 'dueDate', label: 'الاستحقاق' },
  { key: 'total', label: 'الإجمالي', numeric: true },
  { key: 'outstanding', label: 'المتبقي', numeric: true },
  { key: 'actions', label: '', noPrint: true, align: 'end' },
];

const documentColumns: Column<any>[] = [
  { key: 'number', label: 'رقم المستند', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'status', label: 'الحالة' },
  { key: 'grandTotal', label: 'الإجمالي', numeric: true },
  { key: 'outstanding', label: 'المتبقي', numeric: true },
];

const paymentColumns: Column<any>[] = [
  { key: 'number', label: 'رقم السند', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'allocationStatus', label: 'التخصيص' },
  { key: 'amount', label: 'المبلغ', numeric: true },
];

const payLink = (docId?: string): AppRoute => ({
  name: 'payment-new',
  query: { type: isCustomer.value ? 'RECEIVED' : 'PAID', party: id, ref: docId },
});

const agingDocColumns: Column<AgingBucket['documents'][number]>[] = [
  { key: 'number', label: 'المستند' },
  { key: 'date', label: 'التاريخ', type: 'date' },
  { key: 'dueDate', label: 'الاستحقاق', type: 'date' },
  { key: 'outstanding', label: 'المتبقي', type: 'money' },
];

const phones = computed(() => p.value?.phones?.length ? p.value.phones : p.value?.phone ? [{ id: 'legacy', label: 'mobile' as const, number: p.value.phone }] : []);
function whatsappHref(number: string) {
  return `https://wa.me/${number.replace(/\D/g, '')}`;
}
</script>

<template>
  <div>
    <ErrorState v-if="party.error.value" :message="party.error.value" @retry="party.reload" />
    <DetailPage
      v-else
      :title="p?.name ?? '…'"
      :status="p && !p.active ? { label: 'موقوف', tone: 'neutral' } : undefined"
      :chips="[
        { label: isCustomer ? ((p as Customer)?.type === 'company' ? 'عميل' : 'عميل فرد') : 'مورد', value: (isCustomer ? ((p as Customer)?.type === 'company' ? 'منشأة' : 'فرد') : (p?.code ?? '—')) },
      ]"
      :back="base"
      :tabs="tabs"
    >
      <template #actions>
        <AppButton v-if="auth.can('reports')" :icon="FileText" :to="{ name: 'report-ledger', query: { [isCustomer ? 'customer' : 'supplier']: id } }">كشف حساب للطباعة</AppButton>
        <AppButton v-if="auth.can('parties', 'write')" :icon="Pencil" :to="partyRoute(kind, 'edit', id)">تعديل</AppButton>
        <AppButton v-if="auth.can('payments', 'write') && (p?.balance ?? 0) > 0" variant="primary" :icon="HandCoins" :to="payLink()">
          {{ isCustomer ? 'تحصيل دفعة' : 'سداد دفعة' }}
        </AppButton>
      </template>

      <template #aside>
        <InsightHints v-if="isCustomer" :rule-keys="CUSTOMER_INSIGHT_RULES" :entity-id="id" />

          <AppCard padding="sm">
            <p class="text-xs text-text-secondary">{{ isCustomer ? 'الرصيد المستحق علينا تحصيله' : 'الرصيد المستحق للمورد' }}</p>
            <SkeletonBlock v-if="!p" class="mt-2" height="h-8" />
            <p v-else class="mt-1 text-2xl font-semibold" :class="p.balance > 0 && isCustomer ? 'text-warning' : ''">
              <MoneyText :value="p.balance" />
            </p>
            <p v-if="p && (p.unallocatedCredit ?? 0) > 0.005" class="mt-1 text-xs text-success">
              + <span class="num">{{ p.unallocatedCredit!.toFixed(2) }}</span> رصيد غير مخصص (دفعة مقدمة)
            </p>

            <template v-if="isCustomer && p && creditLimit > 0">
              <div class="mt-3 border-t border-border pt-3">
                <div class="mb-1 flex items-center justify-between text-xs">
                  <span class="text-text-secondary">الحد الائتماني</span>
                  <span class="num" :class="overLimit && 'text-danger'">{{ p.balance.toFixed(0) }} / {{ creditLimit.toFixed(0) }}</span>
                </div>
                <div class="h-1.5 overflow-hidden rounded-full bg-surface-hover">
                  <div class="h-full rounded-full" :class="overLimit ? 'bg-danger' : creditPct > 80 ? 'bg-warning' : 'bg-primary'" :style="{ width: `${creditPct}%` }" />
                </div>
              </div>
            </template>

            <div v-if="overdueAmount > 0.005" class="mt-3 flex items-center gap-1.5 border-t border-border pt-3 text-xs text-danger">
              <Clock class="size-3.5" /> متأخر <MoneyText :value="overdueAmount" plain class="font-medium" />
            </div>

            <div class="mt-3 grid grid-cols-2 gap-3 border-t border-border pt-3 text-xs">
              <div>
                <p class="text-text-secondary">{{ isCustomer ? 'عدد الفواتير' : 'أوامر الشراء' }}</p>
                <p class="num mt-0.5 text-body font-medium">{{ formatNumber(totals.documents) }}</p>
              </div>
              <div>
                <p class="text-text-secondary">إجمالي التعامل</p>
                <p class="mt-0.5 text-body font-medium"><MoneyText :value="totals.volume" plain /></p>
              </div>
            </div>
          </AppCard>

          <AppCard title="بيانات التواصل" padding="sm">
            <SkeletonBlock v-if="!p" :lines="3" />
            <ul v-else class="space-y-2.5 text-body">
              <li v-for="ph in phones" :key="ph.id" class="flex items-center justify-between gap-2">
                <span class="flex items-center gap-2">
                  <Phone class="size-3.5 shrink-0 text-text-secondary" />
                  <span class="num">{{ ph.number }}</span>
                  <span class="text-tiny text-text-secondary">({{ PHONE_LABEL[ph.label] }})</span>
                </span>
                <a :href="whatsappHref(ph.number)" target="_blank" rel="noopener" class="shrink-0 text-success hover:opacity-80" title="واتساب">
                  <MessageCircle class="size-3.5" />
                </a>
              </li>
              <li v-if="!phones.length" class="flex items-center gap-2 text-text-secondary"><Phone class="size-3.5" />—</li>
              <li v-if="p.email" class="flex items-center gap-2"><Mail class="size-3.5 text-text-secondary" />{{ p.email }}</li>
              <li v-if="!isCustomer" class="flex items-center gap-2"><UserRound class="size-3.5 text-text-secondary" />{{ (p as Supplier).contactPerson ?? '—' }}</li>
              <li class="flex items-center gap-2"><MapPin class="size-3.5 text-text-secondary" />{{ p.address ?? '—' }}</li>
              <li class="flex items-center gap-2"><ReceiptText class="size-3.5 text-text-secondary" />الرقم الضريبي: <span class="num">{{ p.vatNumber ?? '—' }}</span></li>
              <li v-if="p.linkedPartyId" class="flex items-center gap-2">
                <ClipboardList class="size-3.5 text-text-secondary" />
                <RouterLink :to="partyRoute(isCustomer ? 'supplier' : 'customer', 'detail', p.linkedPartyId)" class="text-primary hover:underline">
                  مرتبط بسجل {{ isCustomer ? 'مورد' : 'عميل' }}
                </RouterLink>
              </li>
            </ul>
          </AppCard>
      </template>

          <!-- نظرة عامة -->
          <template #tab-overview>
          <div class="space-y-4">
            <AppCard title="آخر المستندات" padding="none">
              <DataTable
                :columns="documentColumns"
                :rows="(documents.data.value ?? []).slice(0, 5)"
                :loading="documents.loading.value"
                :error="documents.error.value"
                :page-size="0"
                clickable
                empty-title="لا توجد مستندات بعد"
                @retry="documents.reload"
                @row-click="(r: any) => router.push(isCustomer ? { name: 'invoice', params: { id: r.id } } : { name: 'purchase', params: { id: r.id } })"
              >
                <template #cell-number="{ row }"><span class="num text-primary">{{ row.number }}</span></template>
                <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDate(row.date) }}</span></template>
                <template #cell-status="{ row }"><StatusBadge :tone="docStatus(row.status)?.tone ?? 'neutral'" :label="docStatus(row.status)?.label ?? row.status" /></template>
                <template #cell-grandTotal="{ row }"><MoneyText :value="row.grandTotal" plain /></template>
                <template #cell-outstanding="{ row }"><MoneyText :value="row.outstanding" plain class="font-medium text-warning" /></template>
              </DataTable>
            </AppCard>
            <AppCard v-if="open.data.value?.length" title="مستندات مفتوحة" padding="none">
              <DataTable :columns="openColumns" :rows="open.data.value" :page-size="0" empty-title="لا توجد">
                <template #cell-number="{ row }">
                  <RouterLink :to="isCustomer ? { name: 'invoice', params: { id: row.id } } : { name: 'purchase', params: { id: row.id } }" class="num text-primary hover:underline">{{ row.number }}</RouterLink>
                </template>
                <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDate(row.date) }}</span></template>
                <template #cell-dueDate="{ row }"><span class="num text-text-secondary">{{ row.dueDate ? formatDate(row.dueDate) : '—' }}</span></template>
                <template #cell-total="{ row }"><MoneyText :value="row.total" plain /></template>
                <template #cell-outstanding="{ row }"><MoneyText :value="row.outstanding" class="font-medium text-warning" /></template>
                <template #cell-actions="{ row }">
                  <AppButton v-if="auth.can('payments', 'write')" size="sm" :icon="HandCoins" :to="payLink(row.id)">{{ isCustomer ? 'تحصيل' : 'سداد' }}</AppButton>
                </template>
              </DataTable>
            </AppCard>
          </div>
          </template>

          <!-- المستندات -->
          <template #tab-documents>
          <DataTable
            :columns="documentColumns"
            :rows="documents.data.value"
            :loading="documents.loading.value"
            :error="documents.error.value"
            :page-size="20"
            clickable
            empty-title="لا توجد مستندات"
            @retry="documents.reload"
            @row-click="(r: any) => router.push(isCustomer ? { name: 'invoice', params: { id: r.id } } : { name: 'purchase', params: { id: r.id } })"
          >
            <template #cell-number="{ row }"><span class="num text-primary">{{ row.number }}</span></template>
            <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDate(row.date) }}</span></template>
            <template #cell-status="{ row }"><StatusBadge :tone="docStatus(row.status)?.tone ?? 'neutral'" :label="docStatus(row.status)?.label ?? row.status" /></template>
            <template #cell-grandTotal="{ row }"><MoneyText :value="row.grandTotal" plain /></template>
            <template #cell-outstanding="{ row }"><MoneyText :value="row.outstanding" plain class="font-medium text-warning" /></template>
          </DataTable>
          </template>

          <!-- المدفوعات -->
          <template #tab-payments>
          <DataTable
            :columns="paymentColumns"
            :rows="payments.data.value"
            :loading="payments.loading.value"
            :error="payments.error.value"
            :page-size="20"
            clickable
            empty-title="لا توجد مدفوعات"
            @retry="payments.reload"
            @row-click="(r: any) => router.push({ name: 'payment-detail', params: { id: r.id } })"
          >
            <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
            <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
            <template #cell-allocationStatus="{ row }"><StatusBadge :tone="ALLOCATION_TONE[row.allocationStatus]" :label="ALLOCATION_LABEL[row.allocationStatus]" /></template>
            <template #cell-amount="{ row }"><MoneyText :value="row.amount" /></template>
          </DataTable>
          </template>

          <!-- كشف الحساب -->
          <template #tab-statement>
          <DataTable
            :columns="statementColumns"
            :rows="statement.data.value ? [...statement.data.value].reverse() : undefined"
            :loading="statement.loading.value"
            :error="statement.error.value"
            :page-size="20"
            clickable
            empty-title="لا توجد حركات على هذا الحساب"
            @retry="statement.reload"
            @row-click="(r) => router.push(docLink(r))"
          >
            <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
            <template #cell-number="{ row }"><span class="num text-primary">{{ row.number }}</span></template>
            <template #cell-description="{ row }">{{ row.description }}</template>
            <template #cell-debit="{ row }"><MoneyText :value="row.debit" plain dash-zero /></template>
            <template #cell-credit="{ row }"><MoneyText :value="row.credit" plain dash-zero /></template>
            <template #cell-balance="{ row }"><MoneyText :value="row.balance" plain class="font-medium" /></template>
          </DataTable>
          </template>

          <!-- الأعمار -->
          <template #tab-aging>
          <div class="space-y-3">
            <div v-if="aging.loading.value"><SkeletonBlock :lines="4" height="h-12" /></div>
            <template v-else>
              <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
                <AppCard v-for="b in aging.data.value" :key="b.key" padding="sm">
                  <p class="text-xs text-text-secondary">{{ b.label }}</p>
                  <p class="mt-1 text-lg font-semibold" :class="b.key !== 'current' && b.total > 0 && 'text-danger'"><MoneyText :value="b.total" /></p>
                </AppCard>
              </div>
              <AppCard v-for="b in (aging.data.value ?? []).filter((x) => x.documents.length)" :key="`docs-${b.key}`" :title="b.label" padding="none">
                <DataTable :columns="agingDocColumns" :rows="b.documents" :page-size="0" empty-title="لا توجد">
                  <template #cell-number="{ row }">
                    <RouterLink :to="isCustomer ? { name: 'invoice', params: { id: row.id } } : { name: 'purchase', params: { id: row.id } }" class="num text-primary hover:underline">{{ row.number }}</RouterLink>
                  </template>
                </DataTable>
              </AppCard>
            </template>
          </div>
          </template>

          <!-- المرفقات -->
          <template #tab-attachments>
          <AppCard padding="sm">
            <AttachmentField :owner-ref="ownerRef" />
          </AppCard>
          </template>

          <!-- السجل -->
          <template #tab-history>
          <AppCard padding="none">
            <SkeletonBlock v-if="history.loading.value" :lines="4" class="p-4" />
            <ul v-else-if="history.data.value?.length" class="divide-y divide-border">
              <li v-for="h in history.data.value" :key="h.id" class="flex items-start gap-2.5 px-4 py-3 text-body">
                <HistoryIcon class="mt-0.5 size-3.5 shrink-0 text-text-secondary" />
                <div class="min-w-0">
                  <p>{{ h.message }}</p>
                  <p class="num mt-0.5 text-xs text-text-secondary">{{ formatDateTime(h.date) }}</p>
                </div>
              </li>
            </ul>
            <p v-else class="p-4 text-center text-xs text-text-secondary">لا يوجد سجل بعد</p>
          </AppCard>
          </template>
    </DetailPage>
  </div>
</template>
