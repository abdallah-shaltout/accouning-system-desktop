<script setup lang="ts">
withDefaults(defineProps<{ on?: boolean; tone?: 'white' | 'coral' }>(), { on: true, tone: 'white' })
</script>

<template>
  <!-- rtl-ok: product decision (desktop app doc 18.A2) — the "on" thumb always sits on the right. -->
  <span
    dir="ltr"
    class="relative inline-flex h-4.5 w-8 shrink-0 items-center rounded-full p-0.5 transition-colors"
    :class="on ? (tone === 'coral' ? 'bg-coral-500' : 'bg-white/35') : 'bg-white/20'"
  >
    <span class="size-3.5 rounded-full bg-white shadow-sm transition-transform" :class="on ? 'translate-x-3.5' : ''" />
  </span>
</template>
