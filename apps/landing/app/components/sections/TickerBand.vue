<script setup lang="ts">
import { TICKER } from '~/data/features'
</script>

<template>
  <section aria-label="أبرز إمكانيات ايكوال" class="pt-10 lg:pt-12">
    <LContainer>
      <div class="relative overflow-hidden border-y border-white/25">
        <!-- Two copies of the list; the track slides one copy-width toward reading direction and loops. -->
        <div class="flex w-max animate-marquee items-center hover:[animation-play-state:paused]">
          <ul v-for="copy in 2" :key="copy" class="flex shrink-0 items-center" :aria-hidden="copy === 2">
            <li
              v-for="item in TICKER"
              :key="item.label"
              class="flex h-24 items-center gap-2.5 px-10 font-display text-xl font-medium whitespace-nowrap text-white lg:h-28 lg:px-12 lg:text-[1.375rem]"
            >
              <component :is="item.icon" class="size-6 shrink-0 opacity-90" :stroke-width="1.75" />
              {{ item.label }}
            </li>
          </ul>
        </div>
        <div class="pointer-events-none absolute inset-y-0 inset-s-0 w-16 bg-linear-to-l from-coral-500" />
        <div class="pointer-events-none absolute inset-y-0 inset-e-0 w-16 bg-linear-to-r from-coral-500" />
      </div>
    </LContainer>
  </section>
</template>
