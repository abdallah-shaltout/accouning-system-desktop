<script setup lang="ts">
import type { AppShot } from '~/data/screens'

/**
 * A real Equal screenshot in desktop window chrome. The box takes the image's own aspect ratio
 * (width/height attributes), so it never floats in empty space; place shots that sit side by side
 * from crops with the same ratio (data/screens.ts).
 */
withDefaults(defineProps<{ shot: AppShot; sizes?: string }>(), { sizes: 'xs:100vw md:50vw lg:700px' })
</script>

<template>
  <LWindowMock :title="shot.title" :decorative="false">
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
</template>
