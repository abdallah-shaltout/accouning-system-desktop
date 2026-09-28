<script setup lang="ts">
import type { AccordionFeature } from '~/data/features'

/**
 * The accordion's visual (reference: a warm lifestyle photo with a glass card). Rebuilt as a
 * terracotta light scene holding the matching Equal screen, so it stays on-brand and honest.
 */
defineProps<{ active: AccordionFeature['key']; features: AccordionFeature[] }>()
</script>

<template>
  <div class="scene relative isolate aspect-[531/682] w-full overflow-hidden">
    <div
      v-for="f in features"
      :key="f.key"
      class="absolute inset-0 transition-[opacity,transform] duration-900 ease-out-expo"
      :class="f.key === active ? 'scale-100 opacity-100' : 'pointer-events-none scale-[1.04] opacity-0'"
    >
      <div class="absolute inset-s-[7%] top-[8%] w-[104%] rotate-[3deg]">
        <AppScreenMock :screen="f.key" />
      </div>
    </div>
    <div class="scene-light pointer-events-none absolute inset-0" />
    <div class="absolute inset-e-4 bottom-4 w-[62%] min-w-60 sm:inset-e-7 sm:bottom-7">
      <GlassReceiptCard data-scene-card />
    </div>
  </div>
</template>

<style scoped>
.scene {
  background:
    radial-gradient(70% 60% at 85% 10%, color-mix(in oklab, var(--color-coral-300) 55%, transparent), transparent 70%),
    radial-gradient(90% 70% at 10% 100%, color-mix(in oklab, var(--color-forest-900) 70%, transparent), transparent 70%),
    linear-gradient(160deg, var(--color-coral-500), var(--color-coral-700) 55%, var(--color-forest-800));
}
.scene-light {
  background:
    linear-gradient(180deg, transparent 45%, color-mix(in oklab, var(--color-forest-950) 45%, transparent)),
    radial-gradient(60% 40% at 80% 0%, color-mix(in oklab, white 25%, transparent), transparent 70%);
  mix-blend-mode: multiply;
}
</style>
