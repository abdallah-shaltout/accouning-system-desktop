<script setup lang="ts">
import { APP_NAME_AR } from "@/shared/config/brand";
</script>

<template>
  <div class="flex min-h-screen items-center justify-center bg-surface px-4 py-10">
    <div class="w-full max-w-sm">
      <div class="mb-6 text-center">
        <h1 class="text-heading font-semibold text-text-primary">{{ APP_NAME_AR }}</h1>
      </div>
      <div class="rounded-lg border border-border bg-background p-6 shadow-xs">
        <router-view />
      </div>
    </div>
  </div>
</template>
