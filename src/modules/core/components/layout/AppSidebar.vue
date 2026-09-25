<script setup lang="ts">
import { Store } from '@lucide/vue';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { APP_SHORT } from '@/modules/core/helpers/brand';
import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarRail,
} from '@/modules/core/components/shadcn/sidebar';
import NavMain from './NavMain.vue';

const settings = useSettingsStore();
</script>

<template>
  <Sidebar side="right" collapsible="icon">
    <SidebarHeader>
      <div class="flex h-8 items-center gap-2.5 px-2">
        <img
          v-if="settings.settings?.logo"
          :src="settings.settings.logo"
          alt=""
          class="size-6 shrink-0 rounded-md border border-border bg-background object-contain"
        />
        <div v-else class="flex size-6 shrink-0 items-center justify-center rounded-md bg-primary text-on-primary">
          <Store class="size-3.5" />
        </div>
        <div class="min-w-0 group-data-[collapsible=icon]:hidden">
          <p class="truncate text-body font-semibold leading-tight">{{ settings.settings?.storeName ?? '—' }}</p>
          <p class="truncate text-tiny text-text-secondary leading-tight">{{ APP_SHORT }}</p>
        </div>
      </div>
    </SidebarHeader>
    <SidebarContent role="navigation" aria-label="القائمة الرئيسية">
      <NavMain />
    </SidebarContent>
    <SidebarRail />
  </Sidebar>
</template>
