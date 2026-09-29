<script setup lang="ts">
import type { AccordionFeature } from '~/data/features'

/**
 * The accordion's visual: the real Equal screen for the active feature (captured from the desktop
 * app by scripts/app-shots.py, demo data), in a window on the terracotta light scene. The window
 * bleeds off the end edge so the screen reads at a legible size.
 */
const props = defineProps<{ active: AccordionFeature['key']; features: AccordionFeature[] }>()
const current = computed(() => props.features.find((f) => f.key === props.active) ?? props.features[0]!)
</script>

<template>
  <div class="scene relative isolate aspect-4/3 w-full overflow-hidden rounded-card lg:aspect-auto lg:h-full">
    <!-- One window frame; only the screen inside cross-fades, so no coral bleeds through mid-swap. -->
    <div data-scene-card class="absolute inset-s-[6%] top-[9%] w-[170%] sm:w-[128%] lg:w-[114%]">
      <LWindowMock :title="current.shot.title" :decorative="false">
        <div class="relative bg-white">
          <NuxtImg
            v-for="(f, i) in features"
            :key="f.key"
            :src="`/screens/${f.key}.png`"
            :alt="f.shot.alt"
            width="2560"
            height="2000"
            sizes="xs:170vw sm:128vw lg:1100px"
            loading="lazy"
            class="block h-auto w-full transition-[opacity,transform] duration-700 ease-out-expo"
            :class="[i > 0 && 'absolute inset-0', f.key === active ? 'scale-100 opacity-100' : 'scale-[1.02] opacity-0']"
            :aria-hidden="f.key !== active"
          />
        </div>
      </LWindowMock>
    </div>
    <div class="scene-light pointer-events-none absolute inset-0" />
    <p class="absolute inset-e-4 bottom-4 rounded-full bg-forest-950/75 px-3.5 py-1.5 text-micro text-white backdrop-blur-sm sm:inset-e-6 sm:bottom-6">
      لقطة حقيقية من البرنامج · بيانات تجريبية
    </p>
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
    linear-gradient(180deg, transparent 62%, color-mix(in oklab, var(--color-forest-950) 28%, transparent)),
    radial-gradient(60% 40% at 80% 0%, color-mix(in oklab, white 25%, transparent), transparent 70%);
  mix-blend-mode: multiply;
}
</style>
