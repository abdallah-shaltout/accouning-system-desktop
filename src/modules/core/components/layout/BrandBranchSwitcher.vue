<script setup lang="ts">
/**
 * docs/v2/17-ui-system-rtl-themes.md Phase D — sidebar header: logo + store name, with the active
 * branch as a subtitle. Replaces the old static header block + topbar BranchSwitcher.vue with one
 * shadcn DropdownMenu. A single-branch company (or the switcher not applicable — branches feature
 * off, or this user only has one allowed branch) renders as a static, non-interactive row: no
 * chevron, matching sidebar-07's "single team → no switcher" behavior.
 */
import { onMounted } from 'vue';
import { Building2, ChevronsUpDown, Globe, Store } from '@lucide/vue';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { ALL_BRANCHES, useBranchStore } from '@/modules/settings/controllers/useBranchStore';
import { APP_SHORT } from '@/modules/core/helpers/brand';
import { SidebarMenu, SidebarMenuButton, SidebarMenuItem, useSidebar } from '@/modules/core/components/shadcn/sidebar';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/modules/core/components/shadcn/dropdown-menu';

const settings = useSettingsStore();
const branchStore = useBranchStore();
const { isMobile } = useSidebar();

onMounted(() => {
  branchStore.load();
});

const subtitle = () => (branchStore.showSwitcher ? (branchStore.isAllBranches ? 'كل الفروع' : (branchStore.selectedBranch?.name ?? APP_SHORT)) : APP_SHORT);
</script>

<template>
  <SidebarMenu>
    <SidebarMenuItem>
      <DropdownMenu v-if="branchStore.showSwitcher">
        <DropdownMenuTrigger as-child>
          <SidebarMenuButton size="lg" class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground">
            <div class="flex aspect-square size-8 shrink-0 items-center justify-center rounded-lg bg-primary text-on-primary">
              <img v-if="settings.settings?.logo" :src="settings.settings.logo" alt="" class="size-full rounded-lg object-contain" />
              <Store v-else class="size-4" />
            </div>
            <div class="grid min-w-0 flex-1 text-start leading-tight">
              <span class="truncate font-semibold">{{ settings.settings?.storeName ?? '—' }}</span>
              <span class="truncate text-xs text-text-secondary">{{ subtitle() }}</span>
            </div>
            <ChevronsUpDown class="ms-auto size-4" />
          </SidebarMenuButton>
        </DropdownMenuTrigger>
        <DropdownMenuContent class="w-56" align="start" :side="isMobile ? 'bottom' : 'right'" :side-offset="4">
          <DropdownMenuItem @select="branchStore.select(ALL_BRANCHES)">
            <Globe class="size-4" />
            <span>كل الفروع</span>
            <span class="ms-auto text-tiny text-text-secondary">عرض فقط</span>
          </DropdownMenuItem>
          <DropdownMenuItem v-for="b in branchStore.availableBranches" :key="b.id" @select="branchStore.select(b.id)">
            <Building2 class="size-4" />
            <span>{{ b.name }}</span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <SidebarMenuButton v-else size="lg" class="cursor-default hover:bg-transparent">
        <div class="flex aspect-square size-8 shrink-0 items-center justify-center rounded-lg bg-primary text-on-primary">
          <img v-if="settings.settings?.logo" :src="settings.settings.logo" alt="" class="size-full rounded-lg object-contain" />
          <Store v-else class="size-4" />
        </div>
        <div class="grid min-w-0 flex-1 text-start leading-tight">
          <span class="truncate font-semibold">{{ settings.settings?.storeName ?? '—' }}</span>
          <span class="truncate text-xs text-text-secondary">{{ subtitle() }}</span>
        </div>
      </SidebarMenuButton>
    </SidebarMenuItem>
  </SidebarMenu>
</template>
