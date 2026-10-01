<script setup lang="ts">
import { reactive } from "vue";
import { ChevronDownIcon } from "@lucide/vue";
import { RouterLink } from "vue-router";
import { useAdminSession } from "@/shared/auth/useSession";
import { AppButton } from "@/shared/components/app";
import { adminNavigation } from "@/shared/config/navigation";
import { APP_SHORT } from "@/shared/config/brand";
import { copy } from "@/shared/config/copy";
import { cn } from "@/shared/helpers/utils";

/**
 * Collapsible-group sidebar + top bar (CLAUDE.md desktop-parity "sidebar stays small: collapsible
 * groups (only the active one open)"). D1 placeholder: real admin pages/groups land in D2 via
 * `shared/config/navigation.ts`; this layout just renders whatever is registered there.
 */
const session = useAdminSession();

const openGroups = reactive<Record<string, boolean>>(
  Object.fromEntries(adminNavigation.map((group, i) => [group.group, i === 0])),
);

function toggleGroup(group: string): void {
  openGroups[group] = !openGroups[group];
}

async function logout(): Promise<void> {
  // Real logout call (POST /admin/auth/logout) belongs to the admin auth module's service (D2) —
  // this placeholder only clears the in-memory session.
  session.clear();
}
</script>

<template>
  <div class="flex min-h-screen bg-background">
    <aside class="flex w-56 shrink-0 flex-col border-e border-border bg-surface">
      <div class="px-4 py-4 text-lead font-semibold text-text-primary">{{ APP_SHORT }}</div>
      <nav class="flex-1 space-y-1 overflow-y-auto px-2">
        <div v-for="group in adminNavigation" :key="group.group">
          <button
            type="button"
            class="flex w-full items-center justify-between rounded-md px-2 py-1.5 text-tiny font-medium text-text-secondary hover:bg-surface-hover"
            @click="toggleGroup(group.group)"
          >
            <span>{{ group.group }}</span>
            <ChevronDownIcon :class="cn('size-3.5 transition-transform', openGroups[group.group] && 'rotate-180')" />
          </button>
          <div v-show="openGroups[group.group]" class="mt-0.5 space-y-0.5 ps-2">
            <RouterLink
              v-for="item in group.items"
              :key="item.label"
              :to="item.to"
              class="flex items-center gap-2 rounded-md px-2 py-1.5 text-body text-text-primary hover:bg-surface-hover"
              active-class="bg-surface-hover font-medium"
            >
              <component :is="item.icon" v-if="item.icon" class="size-4" />
              <span>{{ item.label }}</span>
            </RouterLink>
          </div>
        </div>
      </nav>
      <div class="border-t border-border p-3">
        <div v-if="session.user" class="mb-2 truncate text-body text-text-secondary">
          {{ session.user.name ?? session.user.email ?? session.user.id }}
        </div>
        <AppButton variant="outline" size="sm" class="w-full" @click="logout">{{ copy.actions.logout }}</AppButton>
      </div>
    </aside>
    <main class="flex-1 overflow-y-auto px-6 py-6">
      <router-view />
    </main>
  </div>
</template>
