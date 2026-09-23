<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { Plus, UserCog } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatPercent } from '@/modules/core/helpers/format';
import { ROLE_LABEL } from '@/modules/core/helpers/labels';
import { getPriceLists } from '@/modules/products/services/catalogService';
import { useAuthStore } from '../controllers/useAuthStore';
import { getUsers } from '../services/userService';
import type { Role, User } from '../types';

const router = useRouter();
const auth = useAuthStore();
const search = ref('');
const role = ref<Role | 'all'>('all');

const { data, loading, error, reload } = useAsync(async () => {
  const [users, priceLists] = await Promise.all([getUsers(), getPriceLists()]);
  return { users, priceLists };
});

const rows = computed(() =>
  (data.value?.users ?? []).filter(
    (u) =>
      (role.value === 'all' || u.role === role.value) &&
      (!search.value || `${u.name} ${u.username} ${u.phone ?? ''}`.toLowerCase().includes(search.value.toLowerCase())),
  ),
);

const roleOptions = computed(() => [
  { value: 'all' as const, label: 'الكل', count: data.value?.users.length },
  ...(['admin', 'manager', 'accountant', 'cashier'] as Role[]).map((r) => ({
    value: r,
    label: ROLE_LABEL[r],
    count: data.value?.users.filter((u) => u.role === r).length,
  })),
]);

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
  <div>
    <PageHeader title="المستخدمين" subtitle="الصلاحيات، حدود الخصم، وقوائم الأسعار لكل مستخدم">
      <template #actions>
        <AppButton variant="primary" :icon="Plus" to="/users/new">مستخدم جديد</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <SegmentedControl v-model="role" :options="roleOptions" />
      <SearchInput v-model="search" placeholder="بحث بالاسم أو اسم المستخدم" />
    </div>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="UserCog"
      @retry="reload"
      @row-click="(u) => router.push(`/users/${u.id}`)"
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
  </div>
</template>
