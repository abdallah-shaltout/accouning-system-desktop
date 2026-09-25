<script setup lang="ts">
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
} from '@/modules/core/components/shadcn/sidebar';
import BrandBranchSwitcher from './BrandBranchSwitcher.vue';
import NavMain from './NavMain.vue';
import NavQuickActions from './NavQuickActions.vue';
import NavUser from './NavUser.vue';
</script>

<template>
  <Sidebar side="right" collapsible="icon">
    <SidebarHeader>
      <BrandBranchSwitcher />
    </SidebarHeader>
    <SidebarContent role="navigation" aria-label="القائمة الرئيسية">
      <NavMain />
      <NavQuickActions />
    </SidebarContent>
    <SidebarFooter>
      <NavUser />
    </SidebarFooter>
    <SidebarRail />
  </Sidebar>
</template>
