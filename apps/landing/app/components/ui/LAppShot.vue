<script setup lang="ts">
import type { AppShot } from '~/data/screens'

/**
 * A real Equal screenshot in desktop window chrome. `bleed` makes the window wider than its box so
 * the screen stays legible; it overflows toward the end edge and the parent clips it.
 */
withDefaults(defineProps<{ shot: AppShot; bleed?: number; sizes?: string }>(), { bleed: 1, sizes: 'xs:100vw md:50vw lg:700px' })
</script>

<template>
  <div class="w-full">
    <LWindowMock :title="shot.title" :decorative="false" :style="{ width: `${bleed * 100}%` }">
      <NuxtImg
        :src="shot.src"
        :alt="shot.alt"
        :width="shot.width"
        :height="shot.height"
        :sizes="sizes"
        loading="lazy"
        class="block h-auto w-full"
      />
    </LWindowMock>
  </div>
</template>
