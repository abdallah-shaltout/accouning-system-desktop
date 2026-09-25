<script setup lang="ts">
/**
 * docs/v2/17-ui-system-rtl-themes.md Phase D — "إجراءات سريعة" list below the nav groups (the
 * sidebar-07 "Projects" slot). Filtered by permission; hidden entirely in icon mode (no flyout —
 * these are shortcuts, not navigation the user needs to find while collapsed).
 */
import { computed } from 'vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { SidebarGroup, SidebarGroupLabel, SidebarMenu, SidebarMenuButton, SidebarMenuItem, useSidebar } from '@/modules/core/components/shadcn/sidebar';
import { QUICK_ACTIONS } from '../../helpers/navigation';

const auth = useAuthStore();
const { state } = useSidebar();

const actions = computed(() => QUICK_ACTIONS.filter((a) => auth.can(a.area, 'write')));
</script>

<template>
  <SidebarGroup v-if="actions.length && state === 'expanded'">
    <SidebarGroupLabel>إجراءات سريعة</SidebarGroupLabel>
    <SidebarMenu>
      <SidebarMenuItem v-for="action in actions" :key="action.to">
        <SidebarMenuButton as-child size="sm">
          <RouterLink :to="action.to">
            <component :is="action.icon" />
            <span>{{ action.label }}</span>
          </RouterLink>
        </SidebarMenuButton>
      </SidebarMenuItem>
    </SidebarMenu>
  </SidebarGroup>
</template>
