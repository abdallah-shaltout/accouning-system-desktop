<script setup lang="ts">
/** Desktop-app window chrome (Equal is a Windows app, so screens sit in a window, not a phone). */
/** `decorative: false` when the window holds a real screenshot whose alt text should be read. */
withDefaults(defineProps<{ title?: string; theme?: 'light' | 'dark'; decorative?: boolean }>(), {
  title: APP_NAME_AR,
  theme: 'light',
  decorative: true,
})
</script>

<template>
  <div
    class="overflow-hidden rounded-mini border shadow-float"
    :class="theme === 'dark' ? 'border-white/10 bg-forest-950 text-white' : 'border-panel-200 bg-white text-ink'"
    :aria-hidden="decorative ? 'true' : undefined"
  >
    <div
      class="flex h-8 items-center justify-between border-b px-3"
      :class="theme === 'dark' ? 'border-white/10 bg-forest-900' : 'border-panel-200 bg-panel-50'"
    >
      <span class="flex items-center gap-1.5 text-micro" :class="theme === 'dark' ? 'text-white/60' : 'text-ink-soft'">
        <LLogo :tone="theme === 'dark' ? 'white' : 'coral'" class="text-[9px]" />
        {{ title }}
      </span>
      <span class="flex items-center gap-3" :class="theme === 'dark' ? 'text-white/40' : 'text-ink-mute'">
        <span class="h-px w-2.5 bg-current" />
        <span class="size-2.5 rounded-[2px] border border-current" />
        <svg class="size-2.5" viewBox="0 0 10 10"><path d="M1 1l8 8M9 1L1 9" stroke="currentColor" stroke-width="1.2" /></svg>
      </span>
    </div>
    <slot />
  </div>
</template>
