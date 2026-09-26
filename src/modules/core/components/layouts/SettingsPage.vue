<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 (F1 layouts table — "SettingsPage"): nav list · section content. Settings
 * already has a shared nav (`modules/settings/components/SettingsTabs.vue`, route-driven top tabs) —
 * this layout doesn't duplicate that nav, it just gives every settings page the same header +
 * "nav below header, content below nav" shell, with the nav passed via the `#nav` slot so pages keep
 * using `SettingsTabs` as-is.
 *
 * Not wired into any existing settings page yet (F-5 migrates them — out of scope for F-0).
 */
defineProps<{ title: string; subtitle?: string }>();
</script>

<template>
  <div>
    <header class="mb-1">
      <h1 class="text-lg font-semibold tracking-tight">{{ title }}</h1>
      <p v-if="subtitle" class="mt-0.5 text-body text-text-secondary">{{ subtitle }}</p>
    </header>

    <slot name="nav" />

    <div class="max-w-3xl">
      <slot />
    </div>
  </div>
</template>
