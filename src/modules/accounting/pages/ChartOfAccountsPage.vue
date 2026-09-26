<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import {
    ChevronDown,
    ChevronsDownUp,
    ChevronsUpDown,
    FileText,
    Folder,
    FolderOpen,
    Lock,
    Pencil,
    Plus,
    Trash,
} from "@lucide/vue";
import AppButton from "@/modules/core/components/ui/AppButton.vue";
import AppSwitch from "@/modules/core/components/ui/AppSwitch.vue";
import DateRangeFilter from "@/modules/core/components/ui/DateRangeFilter.vue";
import DirIcon from "@/modules/core/components/ui/DirIcon.vue";
import ErrorState from "@/modules/core/components/ui/ErrorState.vue";
import MoneyText from "@/modules/core/components/ui/MoneyText.vue";
import PageHeader from "@/modules/core/components/ui/PageHeader.vue";
import { dirIcon } from "@/modules/core/helpers/dirIcon";
import SearchInput from "@/modules/core/components/ui/SearchInput.vue";
import SkeletonBlock from "@/modules/core/components/ui/SkeletonBlock.vue";
import StatusBadge from "@/modules/core/components/ui/StatusBadge.vue";
import { useAsync } from "@/modules/core/controllers/useAsync";
import { useConfirm } from "@/modules/core/controllers/useConfirm";
import { useToast } from "@/modules/core/controllers/useToast";
import { matchesSearch } from "@/modules/core/helpers/search";
import { useAuthStore } from "@/modules/users/controllers/useAuthStore";
import AccountFormModal from "../components/AccountFormModal.vue";
import {
    deleteAccount,
    getAccounts,
    reparentAccount,
    rolledBalance,
    type AccountWithBalance,
} from "../services/accountingService";
import type { AccountKind } from "../types";

const router = useRouter();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const canWrite = computed(() => auth.can("accounting", "write"));

const from = ref("");
const to = ref("");
const {
    data: accounts,
    loading,
    error,
    reload,
} = useAsync(() =>
    getAccounts({ from: from.value || undefined, to: to.value || undefined }),
);
watch([from, to], reload);

const search = ref("");
const showZero = ref(true);
const collapsed = ref(new Set<string>());
const modalOpen = ref(false);
const editing = ref<AccountWithBalance | null>(null);
const preset = ref<{ kind: AccountKind; parentId?: string }>();
const dragId = ref<string>();
const dropTargetId = ref<string>();

interface TreeRow {
    id: string;
    code: string;
    name: string;
    depth: number;
    balance: number;
    hasChildren: boolean;
    account: AccountWithBalance;
}

const rows = computed<TreeRow[]>(() => {
    const list = accounts.value ?? [];
    const q = search.value.trim();
    const out: TreeRow[] = [];

    const isZero = (a: AccountWithBalance) =>
        Math.abs(rolledBalance(a, list)) < 0.005 && !a.hasPostings;
    const matches = (a: AccountWithBalance): boolean =>
        (matchesSearch([a.code, a.name], q) ||
            list.some((c) => c.parentId === a.id && matches(c))) &&
        (showZero.value || a.isGroup || !isZero(a));

    const walk = (parentId: string | null, depth: number) => {
        for (const a of list
            .filter((x) => (x.parentId ?? null) === parentId && matches(x))
            .sort((x, y) => x.code.localeCompare(y.code))) {
            const children = list.some((c) => c.parentId === a.id);
            out.push({
                id: a.id,
                code: a.code,
                name: a.name,
                depth,
                balance: rolledBalance(a, list),
                hasChildren: children,
                account: a,
            });
            if (children && (q || !collapsed.value.has(a.id)))
                walk(a.id, depth + 1);
        }
    };
    walk(null, 0);
    return out;
});

function toggle(id: string) {
    const next = new Set(collapsed.value);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    collapsed.value = next;
}

function collapseAll() {
    collapsed.value = new Set(
        (accounts.value ?? []).filter((a) => a.isGroup).map((a) => a.id),
    );
}

function openNew(kind: AccountKind, parentId?: string) {
    editing.value = null;
    preset.value = { kind, parentId };
    modalOpen.value = true;
}

function openEdit(a: AccountWithBalance) {
    editing.value = a;
    modalOpen.value = true;
}

async function remove(a: AccountWithBalance) {
    const ok = await confirm({
        title: `حذف الحساب ${a.code} — ${a.name}؟`,
        confirmText: "حذف",
        danger: true,
    });
    if (!ok) return;
    try {
        await deleteAccount(a.id);
        toast.success("تم حذف الحساب");
        reload();
    } catch (err) {
        toast.error(err);
    }
}

// --- Drag to re-parent (docs/v2/03-chart-of-accounts.md §6: kind-change validation on the server) ---
function onDragStart(row: TreeRow) {
    if (!canWrite.value || !row.account.canDelete) return;
    dragId.value = row.id;
}
function onDragOver(row: TreeRow, e: DragEvent) {
    if (!dragId.value || dragId.value === row.id || !row.account.isGroup)
        return;
    e.preventDefault();
    dropTargetId.value = row.id;
}
async function onDrop(row: TreeRow) {
    const id = dragId.value;
    dragId.value = undefined;
    dropTargetId.value = undefined;
    if (!id || id === row.id || !row.account.isGroup) return;
    try {
        await reparentAccount(id, row.id);
        toast.success("تم نقل الحساب");
        reload();
    } catch (err) {
        toast.error(err);
    }
}
</script>

<template>
    <div>
        <PageHeader
            title="شجرة الحسابات"
            subtitle="شجرة الحسابات بالرموز والأرصدة — الحسابات الرئيسية (التجميعية) لا تقبل الترحيل المباشر، والمقفلة أساسية يعتمد عليها الترحيل الآلي"
        >
            <template v-if="canWrite" #actions>
                <AppButton
                    variant="primary"
                    :icon="Plus"
                    @click="openNew('ASSET')"
                    >حساب جديد</AppButton
                >
            </template>
        </PageHeader>

        <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
            <div class="flex flex-wrap items-center gap-3">
                <div class="flex gap-1.5">
                    <AppButton
                        size="sm"
                        variant="ghost"
                        :icon="ChevronsUpDown"
                        @click="collapsed = new Set()"
                        >توسيع الكل</AppButton
                    >
                    <AppButton
                        size="sm"
                        variant="ghost"
                        :icon="ChevronsDownUp"
                        @click="collapseAll"
                        >طي الكل</AppButton
                    >
                </div>
                <AppSwitch
                    v-model="showZero"
                    label="إظهار الأرصدة الصفرية"
                    class="text-xs"
                />
            </div>
            <div class="flex flex-wrap items-center gap-3">
                <DateRangeFilter v-model:from="from" v-model:to="to" />
                <SearchInput
                    v-model="search"
                    placeholder="بحث بالرمز أو الاسم"
                />
            </div>
        </div>

        <ErrorState v-if="error" :message="error" @retry="reload" />
        <div v-else class="overflow-hidden rounded-xl border border-border">
            <div
                class="grid grid-cols-[1fr_90px_160px_120px] items-center gap-3 border-b border-border bg-surface px-4 py-2.5 text-xs font-medium text-text-secondary"
            >
                <span>الحساب</span>
                <span>الطبيعة</span>
                <span>الرصيد</span>
                <span />
            </div>
            <div v-if="loading && !accounts" class="p-4">
                <SkeletonBlock :lines="12" />
            </div>
            <div
                v-for="row in rows"
                v-else
                :key="row.id"
                class="group grid grid-cols-[1fr_90px_160px_120px] items-center gap-3 border-b border-border px-4 last:border-0"
                :class="[
                    row.account.isGroup
                        ? 'h-11 bg-surface/60 font-semibold'
                        : 'h-10 hover:bg-surface-hover',
                    dropTargetId === row.id &&
                        'bg-primary/10 ring-1 ring-inset ring-primary/40',
                ]"
                :draggable="canWrite && row.account.canDelete"
                @dragstart="onDragStart(row)"
                @dragover="onDragOver(row, $event)"
                @dragleave="
                    dropTargetId === row.id && (dropTargetId = undefined)
                "
                @drop="onDrop(row)"
            >
                <div
                    class="flex min-w-0 items-center gap-1.5"
                    :style="{ paddingInlineStart: `${row.depth * 22}px` }"
                >
                    <button
                        v-if="row.hasChildren"
                        type="button"
                        class="rounded p-0.5 text-text-secondary hover:bg-surface-hover"
                        :aria-label="collapsed.has(row.id) ? 'توسيع' : 'طي'"
                        @click="toggle(row.id)"
                    >
                        <DirIcon
                            v-if="collapsed.has(row.id) && !search"
                            :icon="dirIcon.open"
                            class="size-4"
                        />
                        <ChevronDown v-else class="size-4" />
                    </button>
                    <span v-else class="w-5" />
                    <component
                        :is="
                            row.account.isGroup
                                ? collapsed.has(row.id)
                                    ? Folder
                                    : FolderOpen
                                : FileText
                        "
                        class="size-4 shrink-0 text-text-secondary"
                        :stroke-width="1.5"
                    />
                    <span
                        class="num shrink-0 text-text-secondary"
                        :class="!row.account.isGroup && 'text-xs'"
                        >{{ row.code }}</span
                    >
                    <span
                        class="truncate text-body"
                        :class="
                            !row.account.active &&
                            'text-text-secondary line-through'
                        "
                        >{{ row.name }}</span
                    >
                    <Lock
                        v-if="!row.account.canDelete"
                        class="size-3 shrink-0 text-text-secondary/60"
                        aria-label="حساب أساسي"
                    />
                    <StatusBadge
                        v-if="row.account.requiresParty"
                        label="حساب ضبط"
                        tone="neutral"
                        class="ms-1"
                    />
                    <StatusBadge
                        v-if="!row.account.active"
                        label="موقوف"
                        class="ms-1"
                    />
                </div>
                <span class="text-xs text-text-secondary">{{
                    row.account.normalSide === "DEBIT" ? "مدين" : "دائن"
                }}</span>
                <button
                    type="button"
                    class="text-start disabled:cursor-default"
                    :disabled="row.account.isGroup || !auth.can('reports')"
                    :title="!row.account.isGroup ? 'عرض كشف الحساب' : undefined"
                    @click="router.push({ name: 'report-ledger', query: { account: row.id } })"
                >
                    <MoneyText
                        :value="row.balance"
                        :class="
                            !row.account.isGroup &&
                            auth.can('reports') &&
                            'hover:text-primary'
                        "
                        dash-zero
                    />
                </button>
                <div
                    v-if="canWrite"
                    class="flex justify-end gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100"
                >
                    <button
                        type="button"
                        class="rounded p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
                        title="حساب فرعي"
                        aria-label="إضافة حساب فرعي"
                        @click="
                            row.account.isGroup
                                ? openNew(row.account.kind, row.id)
                                : openNew(
                                      row.account.kind,
                                      row.account.parentId ?? undefined,
                                  )
                        "
                    >
                        <Plus class="size-3.5" />
                    </button>
                    <button
                        type="button"
                        class="rounded p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
                        aria-label="تعديل"
                        @click="openEdit(row.account)"
                    >
                        <Pencil class="size-3.5" />
                    </button>
                    <button
                        v-if="row.account.canDelete"
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
            v-if="accounts"
            v-model:open="modalOpen"
            :accounts="accounts"
            :account="editing"
            :preset="preset"
            @saved="reload"
        />
    </div>
</template>
