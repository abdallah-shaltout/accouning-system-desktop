<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { ChevronDown, ChevronLeft, ChevronsDownUp, ChevronsUpDown, FileText, Folder, FolderOpen, Lock, Pencil, Plus, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import AccountFormModal from '../components/AccountFormModal.vue';
import { deleteAccount, getAccountGroups, getAccounts, type AccountWithBalance } from '../services/accountingService';
import type { AccountGroup } from '../types';

const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const canWrite = computed(() => auth.can('accounting', 'write'));

const { data, loading, error, reload } = useAsync(async () => {
  const [groups, accounts] = await Promise.all([getAccountGroups(), getAccounts()]);
  return { groups, accounts };
});

const search = ref('');
const collapsed = ref(new Set<string>());
const modalOpen = ref(false);
const editing = ref<AccountWithBalance | null>(null);
const preset = ref<{ groupId: string; parentId?: string }>();

interface TreeRow {
  kind: 'group' | 'account';
  id: string;
  code: string;
  name: string;
  depth: number;
  balance: number;
  hasChildren: boolean;
  account?: AccountWithBalance;
  group?: AccountGroup;
}

/** Balance of an account including its sub-accounts, in the account's normal direction. */
function rolledBalance(account: AccountWithBalance, all: AccountWithBalance[]): number {
  return all
    .filter((c) => c.parentId === account.id)
    .reduce((acc, c) => acc + (c.normalSide === account.normalSide ? 1 : -1) * rolledBalance(c, all), account.balance);
}

const rows = computed<TreeRow[]>(() => {
  if (!data.value) return [];
  const { groups, accounts } = data.value;
  const q = search.value.trim().toLowerCase();
  const out: TreeRow[] = [];

  const matches = (a: AccountWithBalance): boolean =>
    !q || `${a.code} ${a.name}`.toLowerCase().includes(q) || accounts.some((c) => c.parentId === a.id && matches(c));

  const walk = (parentId: string | undefined, groupId: string, depth: number) => {
    for (const a of accounts.filter((x) => x.groupId === groupId && (x.parentId ?? undefined) === parentId && matches(x))) {
      const children = accounts.some((c) => c.parentId === a.id);
      out.push({ kind: 'account', id: a.id, code: a.code, name: a.name, depth, balance: rolledBalance(a, accounts), hasChildren: children, account: a });
      if (children && (q || !collapsed.value.has(a.id))) walk(a.id, groupId, depth + 1);
    }
  };

  for (const g of groups) {
    const top = accounts.filter((a) => a.groupId === g.id && !a.parentId);
    const balance = top.reduce((acc, a) => acc + (a.normalSide === g.normalSide ? 1 : -1) * rolledBalance(a, accounts), 0);
    if (q && !top.some(matches)) continue;
    out.push({ kind: 'group', id: g.id, code: g.code, name: g.name, depth: 0, balance, hasChildren: top.length > 0, group: g });
    if (q || !collapsed.value.has(g.id)) walk(undefined, g.id, 1);
  }
  return out;
});

function toggle(id: string) {
  const next = new Set(collapsed.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  collapsed.value = next;
}

function collapseAll() {
  collapsed.value = new Set(data.value?.groups.map((g) => g.id));
}

function openNew(groupId: string, parentId?: string) {
  editing.value = null;
  preset.value = { groupId, parentId };
  modalOpen.value = true;
}

function openEdit(a: AccountWithBalance) {
  editing.value = a;
  modalOpen.value = true;
}

async function remove(a: AccountWithBalance) {
  const ok = await confirm({ title: `حذف الحساب ${a.code} — ${a.name}؟`, confirmText: 'حذف', danger: true });
  if (!ok) return;
  try {
    await deleteAccount(a.id);
    toast.success('تم حذف الحساب');
    reload();
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="دليل الحسابات" subtitle="شجرة الحسابات بالرموز والأرصدة الحالية — الحسابات المقفلة أساسية يعتمد عليها الترحيل الآلي">
      <template v-if="canWrite" #actions>
        <AppButton variant="primary" :icon="Plus" @click="openNew(data?.groups[0]?.id ?? '')">حساب جديد</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <div class="flex gap-1.5">
        <AppButton size="sm" variant="ghost" :icon="ChevronsUpDown" @click="collapsed = new Set()">توسيع الكل</AppButton>
        <AppButton size="sm" variant="ghost" :icon="ChevronsDownUp" @click="collapseAll">طي الكل</AppButton>
      </div>
      <SearchInput v-model="search" placeholder="بحث بالرمز أو الاسم" />
    </div>

    <ErrorState v-if="error" :message="error" @retry="reload" />
    <div v-else class="overflow-hidden rounded-xl border border-border">
      <div class="grid grid-cols-[1fr_90px_160px_120px] items-center gap-3 border-b border-border bg-surface px-4 py-2.5 text-xs font-medium text-text-secondary">
        <span>الحساب</span>
        <span>الطبيعة</span>
        <span>الرصيد</span>
        <span />
      </div>
      <div v-if="loading && !data" class="p-4"><SkeletonBlock :lines="12" /></div>
      <div
        v-for="row in rows"
        v-else
        :key="row.id"
        class="group grid grid-cols-[1fr_90px_160px_120px] items-center gap-3 border-b border-border px-4 last:border-0"
        :class="row.kind === 'group' ? 'h-11 bg-surface/60 font-semibold' : 'h-10 hover:bg-surface-hover'"
      >
        <div class="flex min-w-0 items-center gap-1.5" :style="{ paddingInlineStart: `${row.depth * 22}px` }">
          <button
            v-if="row.hasChildren"
            type="button"
            class="rounded p-0.5 text-text-secondary hover:bg-surface-hover"
            :aria-label="collapsed.has(row.id) ? 'توسيع' : 'طي'"
            @click="toggle(row.id)"
          >
            <ChevronLeft v-if="collapsed.has(row.id) && !search" class="size-4" />
            <ChevronDown v-else class="size-4" />
          </button>
          <span v-else class="w-5" />
          <component :is="row.kind === 'group' ? (collapsed.has(row.id) ? Folder : FolderOpen) : FileText" class="size-4 shrink-0 text-text-secondary" :stroke-width="1.5" />
          <span class="num shrink-0 text-text-secondary" :class="row.kind === 'account' && 'text-xs'">{{ row.code }}</span>
          <span class="truncate text-[13px]" :class="row.account && !row.account.active && 'text-text-secondary line-through'">{{ row.name }}</span>
          <Lock v-if="row.account && !row.account.canDelete" class="size-3 shrink-0 text-text-secondary/60" aria-label="حساب أساسي" />
          <StatusBadge v-if="row.account && !row.account.active" label="موقوف" class="ms-1" />
        </div>
        <span class="text-xs text-text-secondary">{{ (row.account ?? row.group)?.normalSide === 'DEBIT' ? 'مدين' : 'دائن' }}</span>
        <button
          type="button"
          class="text-start disabled:cursor-default"
          :disabled="row.kind === 'group' || !auth.can('reports')"
          :title="row.kind === 'account' ? 'عرض كشف الحساب' : undefined"
          @click="router.push(`/reports/ledger?account=${row.id}`)"
        >
          <MoneyText :value="row.balance" :class="row.kind === 'account' && auth.can('reports') && 'hover:text-primary'" dash-zero />
        </button>
        <div v-if="canWrite" class="flex justify-end gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
          <button
            type="button"
            class="rounded p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
            title="حساب فرعي"
            aria-label="إضافة حساب فرعي"
            @click="row.kind === 'group' ? openNew(row.id) : openNew(row.account!.groupId, row.id)"
          >
            <Plus class="size-3.5" />
          </button>
          <button
            v-if="row.account"
            type="button"
            class="rounded p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
            aria-label="تعديل"
            @click="openEdit(row.account)"
          >
            <Pencil class="size-3.5" />
          </button>
          <button
            v-if="row.account?.canDelete"
            type="button"
            class="rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger"
            aria-label="حذف"
            @click="remove(row.account)"
          >
            <Trash class="size-3.5" />
          </button>
        </div>
      </div>
    </div>

    <AccountFormModal
      v-if="data"
      v-model:open="modalOpen"
      :groups="data.groups"
      :accounts="data.accounts"
      :account="editing"
      :preset="preset"
      @saved="reload"
    />
  </div>
</template>
