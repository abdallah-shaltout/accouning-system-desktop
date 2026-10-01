<script setup lang="ts">
import { usePortalSession } from "@/shared/auth/useSession";
import { AppButton } from "@/shared/components/app";
import { APP_NAME_AR } from "@/shared/config/brand";
import { copy } from "@/shared/config/copy";

/** Simple top bar, no sidebar (docs/03-architecture.md — PortalLayout). */
const session = usePortalSession();

async function logout(): Promise<void> {
  // Real logout call (POST /portal/auth/logout) belongs to the portal auth module's service (D3) —
  // this placeholder only clears the in-memory session so the scaffold's guard redirect can be seen.
  session.clear();
}
</script>

<template>
  <div class="flex min-h-screen flex-col bg-background">
    <header class="flex items-center justify-between border-b border-border px-4 py-3">
      <span class="text-lead font-semibold text-text-primary">{{ APP_NAME_AR }}</span>
      <div v-if="session.user" class="flex items-center gap-3">
        <span class="text-body text-text-secondary">{{ session.user.name ?? session.user.id }}</span>
        <AppButton variant="ghost" size="sm" @click="logout">{{ copy.actions.logout }}</AppButton>
      </div>
    </header>
    <main class="mx-auto w-full max-w-3xl flex-1 px-4 py-6">
      <router-view />
    </main>
  </div>
</template>
