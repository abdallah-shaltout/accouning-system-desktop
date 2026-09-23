<script setup lang="ts">
/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §1): topbar branch switcher — "one
 * branch, or كل الفروع (all branches; read views only)". Only rendered at all when
 * `useBranchStore.showSwitcher` is true (feature on AND this user has >1 allowed branch) — see
 * `AppTopbar.vue`'s `v-if`, which is what keeps a single-branch company's topbar unchanged.
 */
import { onBeforeUnmount, ref } from 'vue';
import { Building2, ChevronDown, Globe } from '@lucide/vue';
import { ALL_BRANCHES, useBranchStore } from '@/modules/settings/controllers/useBranchStore';

const branchStore = useBranchStore();
const open = ref(false);
const root = ref<HTMLElement>();

function toggle() {
  open.value = !open.value;
  if (open.value) document.addEventListener('mousedown', onOutside);
  else document.removeEventListener('mousedown', onOutside);
}

function onOutside(e: MouseEvent) {
  if (!root.value?.contains(e.target as Node)) {
    open.value = false;
    document.removeEventListener('mousedown', onOutside);
  }
}

function pick(id: string) {
  branchStore.select(id);
  open.value = false;
}

onBeforeUnmount(() => document.removeEventListener('mousedown', onOutside));
</script>

<template>
  <div ref="root" class="relative">
    <button
      type="button"
      class="flex h-8 items-center gap-1.5 rounded-md border border-border px-2.5 text-xs text-text-secondary hover:bg-surface-hover hover:text-text-primary"
      :aria-expanded="open"
      aria-haspopup="menu"
      @click="toggle"
    >
      <Globe v-if="branchStore.isAllBranches" class="size-3.5" />
      <Building2 v-else class="size-3.5" />
      <span class="max-w-28 truncate">{{ branchStore.isAllBranches ? 'كل الفروع' : branchStore.selectedBranch?.name }}</span>
      <ChevronDown class="size-3" />
    </button>

    <div v-if="open" role="menu" class="absolute end-0 top-full z-50 mt-1.5 w-52 overflow-hidden rounded-lg border border-border bg-background shadow-xl">
      <div class="p-1">
        <button
          type="button"
          role="menuitem"
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-start text-body hover:bg-surface-hover"
          :class="branchStore.isAllBranches ? 'bg-primary/10 text-primary' : ''"
          @click="pick(ALL_BRANCHES)"
        >
          <Globe class="size-3.5" /> كل الفروع
          <span class="ms-auto text-tiny text-text-secondary">عرض فقط</span>
        </button>
        <button
          v-for="b in branchStore.availableBranches"
          :key="b.id"
          type="button"
          role="menuitem"
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-start text-body hover:bg-surface-hover"
          :class="branchStore.selected === b.id ? 'bg-primary/10 text-primary' : ''"
          @click="pick(b.id)"
        >
          <Building2 class="size-3.5" /> {{ b.name }}
        </button>
      </div>
    </div>
  </div>
</template>
