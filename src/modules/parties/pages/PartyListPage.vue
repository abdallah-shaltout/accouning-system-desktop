<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Building, FileText, Truck, Upload, Users } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import FilterBar from '@/modules/core/components/blocks/FilterBar.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import { matchesSearch } from '@/modules/core/helpers/search';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { partyRoute } from '../helpers/partyRoutes';
// v2 phase 5 (docs/v2/05-onboarding.md §5): generic Excel import, wired in here for "import
// customers/suppliers" — the same descriptor pattern the opening-balances step uses.
import ImportWizard from '@/modules/core/components/import/ImportWizard.vue';
import { customersDescriptor, suppliersDescriptor } from '@/modules/core/components/import/descriptors';
import PartyFormModal from '../components/PartyFormModal.vue';
import { getCustomers, getSuppliers } from '../services/partyService';
import type { Customer, Supplier } from '../types';

/** Shared list screen for customers and suppliers (route meta decides which). */
const props = defineProps<{ kind: 'customer' | 'supplier' }>();

type Party = Customer | Supplier;

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const isCustomer = computed(() => props.kind === 'customer');
const canWrite = computed(() => auth.can('parties', 'write'));

const search = computed(() => (typeof route.query.q === 'string' ? route.query.q : ''));
const view = computed<'active' | 'balance' | 'inactive'>(() => (route.query.view === 'balance' || route.query.view === 'inactive' ? route.query.view : 'active'));
const formOpen = ref(false);
const importOpen = ref(false);
const importDescriptor = computed(() => (isCustomer.value ? customersDescriptor : suppliersDescriptor));

const { data, loading, error, reload } = useAsync<Party[]>(() =>
  isCustomer.value ? getCustomers({ includeInactive: true }) : getSuppliers({ includeInactive: true }),
);
watch(() => props.kind, reload);

const rows = computed(() => {
  return (data.value ?? []).filter((p) => {
    if (view.value === 'inactive' ? p.active : !p.active) return false;
    if (view.value === 'balance' && p.balance <= 0) return false;
    return matchesSearch([p.name, p.phone, p.vatNumber, (p as Supplier).contactPerson], search.value);
  });
});

const totalBalance = computed(() => (data.value ?? []).filter((p) => p.active).reduce((a, p) => a + p.balance, 0));

const viewOptions = computed(() => [
  { value: 'active', label: 'النشطون' },
  { value: 'balance', label: isCustomer.value ? 'عليهم رصيد' : 'لهم رصيد' },
  { value: 'inactive', label: 'موقوفون' },
]);

const columns = computed<Column<Party>[]>(() => [
  { key: 'name', label: 'الاسم', sortable: true },
  { key: 'phone', label: 'الهاتف' },
  ...(isCustomer.value ? [] : [{ key: 'contactPerson', label: 'المسؤول' }]),
  { key: 'vatNumber', label: 'الرقم الضريبي' },
  { key: 'balance', label: isCustomer.value ? 'الرصيد المستحق' : 'المستحق للمورد', numeric: true, sortable: true },
]);
</script>

<template>
  <ListPage
    :title="isCustomer ? 'العملاء' : 'الموردين'"
    :subtitle="isCustomer ? 'بيانات العملاء وأرصدتهم المستحقة (الذمم المدينة)' : 'بيانات الموردين والمبالغ المستحقة لهم (الذمم الدائنة)'"
    :primary-action-label="canWrite ? (isCustomer ? 'عميل جديد' : 'مورد جديد') : undefined"
    @primary-action="formOpen = true"
  >
    <template #actions>
      <AppButton v-if="canWrite" :icon="Upload" @click="importOpen = true">استيراد من إكسل</AppButton>
      <AppButton v-if="canWrite" :icon="FileText" :to="partyRoute(kind, 'new')">نموذج كامل</AppButton>
    </template>
    <template #filters>
      <FilterBar search-placeholder="الاسم، الهاتف، أو الرقم الضريبي" :filters="[{ key: 'view', label: 'العرض', options: viewOptions }]" />
    </template>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :export-file-name="isCustomer ? 'العملاء' : 'الموردين'"
      :empty-icon="isCustomer ? Users : Truck"
      :empty-title="isCustomer ? 'لا يوجد عملاء' : 'لا يوجد موردون'"
      @retry="reload"
      @row-click="(p) => router.push(partyRoute(kind, 'detail', p.id))"
    >
      <template #cell-name="{ row }">
        <span class="inline-flex items-center gap-2 font-medium">
          <Building v-if="(row as Customer).type === 'company' || !isCustomer" class="size-3.5 text-text-secondary" />
          {{ row.name }}
        </span>
        <span v-if="row.address" class="block text-xs text-text-secondary">{{ row.address }}</span>
      </template>
      <template #cell-phone="{ row }"><span class="num text-text-secondary">{{ row.phone ?? '—' }}</span></template>
      <template #cell-contactPerson="{ row }"><span class="text-text-secondary">{{ (row as Supplier).contactPerson ?? '—' }}</span></template>
      <template #cell-vatNumber="{ row }"><span class="num text-text-secondary">{{ row.vatNumber ?? '—' }}</span></template>
      <template #cell-balance="{ row }">
        <MoneyText v-if="row.balance > 0" :value="row.balance" class="font-medium" :class="isCustomer ? 'text-warning' : ''" />
        <span v-else class="text-xs text-text-secondary">لا يوجد</span>
        <StatusBadge v-if="!row.active" class="ms-2" label="موقوف" />
      </template>
    </DataTable>

    <p v-if="data?.length" class="mt-3 text-xs text-text-secondary">
      {{ formatNumber(rows.length) }} {{ isCustomer ? 'عميل' : 'مورد' }} · إجمالي {{ isCustomer ? 'المستحق من العملاء' : 'المستحق للموردين' }}
      <MoneyText :value="totalBalance" class="text-text-primary" />
    </p>

    <PartyFormModal v-model:open="formOpen" :kind="kind" @saved="(p) => router.push(partyRoute(kind, 'detail', p.id))" />
    <ImportWizard v-if="importOpen" :descriptor="importDescriptor" @close="importOpen = false" @imported="() => { importOpen = false; reload(); }" />
  </ListPage>
</template>
