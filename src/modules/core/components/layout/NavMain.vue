<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/modules/core/components/shadcn/sidebar';
import { NAVIGATION, type NavItem } from '../../helpers/navigation';

const route = useRoute();
const auth = useAuthStore();

const groups = computed(() =>
  NAVIGATION.map((g) => ({ ...g, items: g.items.filter((i) => auth.can(i.area)) })).filter((g) => g.items.length),
);

function isActive(item: NavItem) {
  if (item.exact) return route.path === item.to;
  return route.path === item.to || route.path.startsWith(`${item.to}/`);
}
</script>

<template>
  <SidebarGroup v-for="(group, gi) in groups" :key="gi">
    <SidebarGroupLabel v-if="group.label">{{ group.label }}</SidebarGroupLabel>
    <SidebarMenu>
      <SidebarMenuItem v-for="item in group.items" :key="item.to">
        <SidebarMenuButton as-child :is-active="isActive(item)" :tooltip="item.label">
          <RouterLink :to="item.to">
            <component :is="item.icon" />
            <span>{{ item.label }}</span>
          </RouterLink>
        </SidebarMenuButton>
      </SidebarMenuItem>
    </SidebarMenu>
  </SidebarGroup>
</template>
