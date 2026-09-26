<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { UserCog } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import FilterBar from '@/modules/core/components/blocks/FilterBar.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatPercent } from '@/modules/core/helpers/format';
import { ROLE_LABEL } from '@/modules/core/helpers/labels';
import { matchesSearch } from '@/modules/core/helpers/search';
import { getPriceLists } from '@/modules/products/services/catalogService';
import { useAuthStore } from '../controllers/useAuthStore';
import { getUsers } from '../services/userService';
import type { Role, User } from '../types';

const router = useRouter();
const route = useRoute();
const auth = useAuthStore();

const search = computed(() => (typeof route.query.q === 'string' ? route.query.q : ''));
const role = computed(() => (typeof route.query.role === 'string' ? (route.query.role as Role) : undefined));

const { data, loading, error, reload } = useAsync(async () => {
  const [users, priceLists] = await Promise.all([getUsers(), getPriceLists()]);
  return { users, priceLists };
});

const rows = computed(() =>
  (data.value?.users ?? []).filter(
    (u) => (!role.value || u.role === role.value) && matchesSearch([u.name, u.username, u.phone], search.value),
  ),
);

const roleOptions = (['admin', 'manager', 'accountant', 'cashier', 'storekeeper'] as Role[]).map((r) => ({
  value: r,
  label: ROLE_LABEL[r],
}));

const priceListName = (id?: string) => data.value?.priceLists.find((p) => p.id === id)?.name ?? 'السعر الأساسي';

const columns: Column<User>[] = [
  { key: 'name', label: 'الاسم', sortable: true },
  { key: 'username', label: 'اسم المستخدم', sortable: true },
  { key: 'role', label: 'الصلاحية', sortable: true },
  { key: 'maxDiscount', label: 'أقصى خصم', numeric: true, sortable: true },
  { key: 'priceListId', label: 'قائمة الأسعار' },
  { key: 'active', label: 'الحالة' },
];
</script>

<template>
  <ListPage
    title="المستخدمين"
    subtitle="الصلاحيات، حدود الخصم، وقوائم الأسعار لكل مستخدم"
    primary-action-label="مستخدم جديد"
    :primary-action-to="{ name: 'user-editor', params: { id: 'new' } }"
  >
    <template #filters>
      <FilterBar search-placeholder="بحث بالاسم أو اسم المستخدم" :filters="[{ key: 'role', label: 'الصلاحية', options: roleOptions }]" />
    </template>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="UserCog"
      @retry="reload"
      @row-click="(u) => router.push({ name: 'user-editor', params: { id: u.id } })"
    >
      <template #cell-name="{ row }">
        <span class="font-medium">{{ row.name }}</span>
        <span v-if="row.id === auth.user?.id" class="ms-2 text-xs text-text-secondary">(أنت)</span>
      </template>
      <template #cell-username="{ row }"><span class="num text-text-secondary">{{ row.username }}</span></template>
      <template #cell-role="{ row }">{{ ROLE_LABEL[row.role] }}</template>
      <template #cell-maxDiscount="{ row }"><span class="num">{{ formatPercent(row.maxDiscount) }}</span></template>
      <template #cell-priceListId="{ row }"><span class="text-text-secondary">{{ priceListName(row.priceListId) }}</span></template>
      <template #cell-active="{ row }">
        <StatusBadge :tone="row.active ? 'success' : 'neutral'" :label="row.active ? 'نشط' : 'موقوف'" />
      </template>
    </DataTable>
  </ListPage>
</template>
