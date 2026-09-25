<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { sidebarCollapsedDefault } from '../../controllers/useAppearance';
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/modules/core/components/shadcn/breadcrumb';
import { Separator } from '@/modules/core/components/shadcn/separator';
import { SidebarInset, SidebarProvider, SidebarTrigger } from '@/modules/core/components/shadcn/sidebar';
import AppSidebar from './AppSidebar.vue';
import AppTopbar from './AppTopbar.vue';
import ErrorBoundary from '../ErrorBoundary.vue';
import KeyboardShortcutsSheet from './KeyboardShortcutsSheet.vue';

const COLLAPSE_KEY = 'app_sidebar_collapsed';
// The "القائمة الجانبية: مطوية افتراضياً" appearance setting seeds this session's initial value;
// once the user toggles it by hand, that per-session choice (below) wins until they clear it.
const sidebarOpen = ref(!sidebarCollapsedDefault.value);
try {
  const saved = localStorage.getItem(COLLAPSE_KEY);
  if (saved !== null) sidebarOpen.value = saved !== '1';
} catch {
  /* ignore */
}
watch(sidebarOpen, (open) => {
  try {
    localStorage.setItem(COLLAPSE_KEY, open ? '0' : '1');
  } catch {
    /* ignore */
  }
});

const route = useRoute();
const crumbs = computed(() => {
  const list: string[] = [];
  if (route.meta.section) list.push(route.meta.section as string);
  if (route.meta.title) list.push(route.meta.title as string);
  return list;
});
</script>

<template>
  <SidebarProvider v-model:open="sidebarOpen">
    <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:start-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-background focus:px-3 focus:py-2 focus:shadow-lg">
      تخطي إلى المحتوى
    </a>
    <AppSidebar />
    <SidebarInset>
      <header class="no-print flex h-[52px] shrink-0 items-center gap-3 border-b border-border bg-background px-4">
        <SidebarTrigger />
        <Separator orientation="vertical" class="h-5" />
        <Breadcrumb class="min-w-0 flex-1">
          <BreadcrumbList>
            <template v-for="(c, i) in crumbs" :key="i">
              <BreadcrumbSeparator v-if="i > 0" />
              <BreadcrumbItem>
                <BreadcrumbPage v-if="i === crumbs.length - 1" class="truncate">{{ c }}</BreadcrumbPage>
                <span v-else class="truncate">{{ c }}</span>
              </BreadcrumbItem>
            </template>
          </BreadcrumbList>
        </Breadcrumb>
        <AppTopbar />
      </header>
      <main id="main-content" class="flex-1 overflow-y-auto">
        <div class="mx-auto w-full max-w-[1400px] px-6 py-6">
          <RouterView v-slot="{ Component, route: r }">
            <ErrorBoundary :key="r.path">
              <component :is="Component" />
            </ErrorBoundary>
          </RouterView>
        </div>
      </main>
    </SidebarInset>
    <KeyboardShortcutsSheet />
  </SidebarProvider>
</template>
