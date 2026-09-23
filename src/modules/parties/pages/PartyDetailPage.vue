<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { FileText, HandCoins, MapPin, Pencil, Phone, ReceiptText, UserRound } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate, formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import { getOpenDocuments } from '@/modules/payments/services/paymentService';
import type { OpenDocument } from '@/modules/payments/types';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import PartyFormModal from '../components/PartyFormModal.vue';
import { getCustomer, getCustomerStatement, getSupplier, getSupplierStatement } from '../services/partyService';
import type { Customer, PartyStatementRow, Supplier } from '../types';

const props = defineProps<{ kind: 'customer' | 'supplier' }>();

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const id = String(route.params.id);
const isCustomer = computed(() => props.kind === 'customer');
const base = computed(() => (isCustomer.value ? '/customers' : '/suppliers'));

const party = useAsync<Customer | Supplier>(() => (isCustomer.value ? getCustomer(id) : getSupplier(id)));
const statement = useAsync(() => (isCustomer.value ? getCustomerStatement(id) : getSupplierStatement(id)));
const open = useAsync(() => getOpenDocuments(props.kind, id));

const tab = ref<'statement' | 'open' | 'attachments'>('statement');
/**
 * Demo usage of `AttachmentField` for this Phase 0 track (docs/v2/14-platform.md §5) — proves the
 * component + IndexedDB blob store + viewer end-to-end. Full wiring of attachments onto the party
 * *form* (national address docs, CR/VAT certs, contact photos) is Phase 4's job — see the TODO
 * there in `PartyFormModal.vue`.
 */
const ownerRef = computed(() => `${props.kind}:${id}`);
const formOpen = ref(false);
const p = computed(() => party.data.value);

const totals = computed(() => {
  const rows = statement.data.value ?? [];
  const docs = rows.filter((r) => r.kind === (isCustomer.value ? 'invoice' : 'purchaseOrder') && (isCustomer.value ? r.debit > 0 : r.credit > 0));
  return {
    documents: docs.length,
    volume: docs.reduce((a, r) => a + r.debit + r.credit, 0),
    lastDate: rows.at(-1)?.date,
  };
});

function docLink(row: PartyStatementRow) {
  if (row.kind === 'invoice' || row.kind === 'refund') return `/invoices/${row.refId}`;
  if (row.kind === 'purchaseOrder' || row.kind === 'purchaseReturn') return `/purchases/${row.refId}`;
  return `/payments?highlight=${row.refId}`;
}

function refresh() {
  party.reload();
  statement.reload();
  open.reload();
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
  { key: 'total', label: 'الإجمالي', numeric: true },
  { key: 'outstanding', label: 'المتبقي', numeric: true },
  { key: 'actions', label: '', noPrint: true, align: 'end' },
];

const payLink = (docId?: string) => ({
  path: '/payments/new',
  query: { type: isCustomer.value ? 'RECEIVED' : 'PAID', party: id, ref: docId },
});
</script>

<template>
  <div>
    <ErrorState v-if="party.error.value" :message="party.error.value" @retry="party.reload" />
    <template v-else>
      <PageHeader :title="p?.name ?? '…'" :back="base">
        <template v-if="p && !p.active" #badge><StatusBadge label="موقوف" /></template>
        <template #subtitle>{{ isCustomer ? ((p as Customer)?.type === 'company' ? 'عميل — منشأة' : 'عميل — فرد') : 'مورد' }}</template>
        <template #actions>
          <AppButton v-if="auth.can('reports')" :icon="FileText" :to="`/reports/ledger?${isCustomer ? 'customer' : 'supplier'}=${id}`">كشف حساب للطباعة</AppButton>
          <AppButton v-if="auth.can('parties', 'write')" :icon="Pencil" @click="formOpen = true">تعديل</AppButton>
          <AppButton v-if="auth.can('payments', 'write') && (p?.balance ?? 0) > 0" variant="primary" :icon="HandCoins" :to="payLink()">
            {{ isCustomer ? 'تحصيل دفعة' : 'سداد دفعة' }}
          </AppButton>
        </template>
      </PageHeader>

      <div class="grid items-start gap-5 xl:grid-cols-[300px_1fr]">
        <div class="space-y-4">
          <AppCard padding="sm">
            <p class="text-xs text-text-secondary">{{ isCustomer ? 'الرصيد المستحق علينا تحصيله' : 'الرصيد المستحق للمورد' }}</p>
            <SkeletonBlock v-if="!p" class="mt-2" height="h-8" />
            <p v-else class="mt-1 text-2xl font-semibold" :class="p.balance > 0 && isCustomer ? 'text-warning' : ''">
              <MoneyText :value="p.balance" />
            </p>
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
              <li class="flex items-center gap-2"><Phone class="size-3.5 text-text-secondary" /><span class="num">{{ p.phone ?? '—' }}</span></li>
              <li v-if="!isCustomer" class="flex items-center gap-2"><UserRound class="size-3.5 text-text-secondary" />{{ (p as Supplier).contactPerson ?? '—' }}</li>
              <li class="flex items-center gap-2"><MapPin class="size-3.5 text-text-secondary" />{{ p.address ?? '—' }}</li>
              <li class="flex items-center gap-2"><ReceiptText class="size-3.5 text-text-secondary" />الرقم الضريبي: <span class="num">{{ p.vatNumber ?? '—' }}</span></li>
            </ul>
          </AppCard>
        </div>

        <div>
          <div class="mb-3 flex items-center justify-between">
            <SegmentedControl
              v-model="tab"
              :options="[
                { value: 'statement', label: 'كشف الحساب', count: statement.data.value?.length },
                { value: 'open', label: isCustomer ? 'فواتير مفتوحة' : 'أوامر غير مسددة', count: open.data.value?.length },
                { value: 'attachments', label: 'المرفقات' },
              ]"
            />
          </div>

          <DataTable
            v-if="tab === 'statement'"
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

          <AppCard v-else-if="tab === 'attachments'" padding="sm">
            <AttachmentField :owner-ref="ownerRef" />
          </AppCard>

          <DataTable
            v-else
            :columns="openColumns"
            :rows="open.data.value"
            :loading="open.loading.value"
            :error="open.error.value"
            :page-size="0"
            empty-title="لا توجد مستندات مفتوحة"
            empty-description="كل المستندات مسددة بالكامل"
            @retry="open.reload"
          >
            <template #cell-number="{ row }">
              <RouterLink :to="isCustomer ? `/invoices/${row.id}` : `/purchases/${row.id}`" class="num text-primary hover:underline">{{ row.number }}</RouterLink>
            </template>
            <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDate(row.date) }}</span></template>
            <template #cell-total="{ row }"><MoneyText :value="row.total" plain /></template>
            <template #cell-outstanding="{ row }"><MoneyText :value="row.outstanding" class="font-medium text-warning" /></template>
            <template #cell-actions="{ row }">
              <AppButton v-if="auth.can('payments', 'write')" size="sm" :icon="HandCoins" :to="payLink(row.id)">{{ isCustomer ? 'تحصيل' : 'سداد' }}</AppButton>
            </template>
          </DataTable>
        </div>
      </div>

      <PartyFormModal v-model:open="formOpen" :kind="kind" :party="p" @saved="refresh" />
    </template>
  </div>
</template>
