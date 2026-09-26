<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ChevronRight } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import {
  SidebarGroup,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  useSidebar,
} from '@/modules/core/components/shadcn/sidebar';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/modules/core/components/shadcn/collapsible';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/modules/core/components/shadcn/dropdown-menu';
import { NAVIGATION, type NavItem } from '../../helpers/navigation';

/**
 * docs/v2/17-ui-system-rtl-themes.md Phase D: each group is one accordion row (icon + name +
 * chevron); only the group containing the current route is open, and opening another closes it
 * (single-open accordion, not persisted — "fewer surprises for non-technical users"). A group
 * whose role filter leaves exactly one item renders as a plain link. In icon mode (sidebar
 * collapsed to a rail) a multi-item group instead opens a DropdownMenu flyout on click.
 */
const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const { state } = useSidebar();

const groups = computed(() =>
  NAVIGATION.map((g) => ({ ...g, items: g.items.filter((i) => auth.can(i.area)) })).filter((g) => g.items.length),
);

/** Resolved once per item — a route object has no `.path` of its own to compare against. */
function itemPath(item: NavItem) {
  return router.resolve(item.to).path;
}

function isActive(item: NavItem) {
  const path = itemPath(item);
  if (item.exact) return route.path === path;
  return route.path === path || route.path.startsWith(`${path}/`);
}

function groupContainsRoute(items: NavItem[]) {
  return items.some(isActive);
}

const openGroup = ref<string | undefined>();

function initOpenGroup() {
  const active = groups.value.find((g) => g.label && groupContainsRoute(g.items));
  openGroup.value = active?.label;
}
initOpenGroup();
watch(() => route.path, initOpenGroup);

function toggleGroup(label: string) {
  openGroup.value = openGroup.value === label ? undefined : label;
}
</script>

<template>
  <SidebarGroup>
    <SidebarMenu>
      <template v-for="group in groups" :key="group.label ?? group.items[0].label">
        <!-- Single link: no label (the "الرئيسية" row) or a role filter left exactly one item. -->
        <SidebarMenuItem v-if="!group.label || group.items.length === 1">
          <SidebarMenuButton as-child :is-active="isActive(group.items[0])" :tooltip="group.items[0].label">
            <RouterLink :to="group.items[0].to">
              <component :is="group.items[0].icon" />
              <span>{{ group.items[0].label }}</span>
            </RouterLink>
          </SidebarMenuButton>
        </SidebarMenuItem>

        <!-- Icon mode: a flyout menu instead of an inline accordion (no need to expand the sidebar). -->
        <SidebarMenuItem v-else-if="state === 'collapsed'">
          <DropdownMenu>
            <DropdownMenuTrigger as-child>
              <SidebarMenuButton :is-active="groupContainsRoute(group.items)" :tooltip="group.label">
                <component :is="group.icon" />
                <span>{{ group.label }}</span>
              </SidebarMenuButton>
            </DropdownMenuTrigger>
            <!-- rtl-ok: side="left" opens the flyout toward the content, away from the sidebar's fixed right side -->
            <DropdownMenuContent side="left" align="start" class="min-w-48">
              <DropdownMenuItem v-for="item in group.items" :key="item.label" as-child>
                <RouterLink :to="item.to" class="flex w-full items-center gap-2">
                  <component :is="item.icon" class="size-4" />
                  <span>{{ item.label }}</span>
                </RouterLink>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </SidebarMenuItem>

        <!-- Expanded mode: an accordion row, only the active group open. -->
        <Collapsible v-else :key="`c-${group.label}`" as-child :open="openGroup === group.label" @update:open="() => toggleGroup(group.label!)">
          <SidebarMenuItem>
            <CollapsibleTrigger as-child>
              <SidebarMenuButton class="group/nav-item" :is-active="groupContainsRoute(group.items)" :tooltip="group.label">
                <component :is="group.icon" />
                <span>{{ group.label }}</span>
                <ChevronRight class="ms-auto size-4 shrink-0 rtl:rotate-180 transition-transform group-data-[state=open]/nav-item:rotate-90 rtl:group-data-[state=open]/nav-item:-rotate-90" />
              </SidebarMenuButton>
            </CollapsibleTrigger>
            <CollapsibleContent>
              <SidebarMenuSub>
                <SidebarMenuSubItem v-for="item in group.items" :key="item.label">
                  <SidebarMenuSubButton as-child :is-active="isActive(item)">
                    <RouterLink :to="item.to">
                      <component :is="item.icon" />
                      <span>{{ item.label }}</span>
                    </RouterLink>
                  </SidebarMenuSubButton>
                </SidebarMenuSubItem>
              </SidebarMenuSub>
            </CollapsibleContent>
          </SidebarMenuItem>
        </Collapsible>
      </template>
    </SidebarMenu>
  </SidebarGroup>
</template>
