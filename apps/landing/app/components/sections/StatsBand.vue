<script setup lang="ts">
import { HERO_STATS } from '~/data/stats'

const root = ref<HTMLElement | null>(null)
useLandingMotion(root, ({ gsap }) => {
  gsap.from('[data-stat-rule]', { scaleY: 0, transformOrigin: 'top', duration: 1, stagger: 0.12, scrollTrigger: revealTrigger(root.value!) })
  gsap.from('[data-stat-label]', { autoAlpha: 0, y: 14, duration: 0.8, stagger: 0.12, delay: 0.4, scrollTrigger: revealTrigger(root.value!) })
})
</script>

<template>
  <section ref="root" aria-label="ايكوال بالأرقام" class="pt-24 lg:pt-44">
    <LContainer>
      <ul class="grid border-b border-white/25 sm:grid-cols-3">
        <li
          v-for="(stat, i) in HERO_STATS"
          :key="stat.label"
          class="relative flex flex-col items-center justify-between gap-6 py-10 text-center text-white sm:py-0 sm:pt-8 sm:pb-14 lg:h-73"
        >
          <span v-if="i > 0" data-stat-rule class="absolute inset-y-0 inset-s-0 hidden w-px bg-white/25 sm:block" />
          <span v-if="i > 0" class="absolute inset-x-0 top-0 h-px bg-white/25 sm:hidden" />
          <p data-stat-label class="order-2 text-sm font-medium text-white/90">{{ stat.label }}</p>
          <p class="order-1 font-display text-stat font-normal tracking-tight">
            <LStatValue :stat="stat" affix-class="opacity-50" />
          </p>
        </li>
      </ul>
    </LContainer>
  </section>
</template>
