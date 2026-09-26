<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 (F1 layouts table — "SettingsPage"): nav list · section content. Settings
 * already has a shared nav (`modules/settings/components/SettingsTabs.vue`, route-driven top tabs) —
 * this layout doesn't duplicate that nav, it just gives every settings page the same header +
 * "nav below header, content below nav" shell, with the nav passed via the `#nav` slot so pages keep
 * using `SettingsTabs` as-is.
 *
 * F-5 (doc 17 Phase F, migration batch): wired into every settings page. Most pages are a single
 * narrow column of cards (`max-w-3xl`, the default). A few (General, Backup, Printing, Products
 * settings) pair a form/list with a sticky aside (logo preview, status card, print preview, unit
 * presets) in a wider `xl:grid-cols-[1fr_360px]`-style layout — `wide` drops the width cap for those
 * so the aside has room, while every narrow page keeps the same fixed reading width as before.
 */
defineProps<{ title: string; subtitle?: string; wide?: boolean }>();
</script>

<template>
  <div>
    <header class="mb-1">
      <h1 class="text-lg font-semibold tracking-tight">{{ title }}</h1>
      <p v-if="subtitle" class="mt-0.5 text-body text-text-secondary">{{ subtitle }}</p>
    </header>

    <slot name="nav" />

    <div :class="wide ? '' : 'max-w-3xl'">
      <slot />
    </div>
  </div>
</template>
