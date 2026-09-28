<script setup lang="ts">
import { TEAM_STATS } from '~/data/stats'

const root = ref<HTMLElement | null>(null)

useLandingMotion(root, ({ gsap }) => {
  const el = root.value!
  gsap.from('[data-reveal="eyebrow"], [data-trust-sub]', { autoAlpha: 0, y: 16, stagger: 0.15, scrollTrigger: revealTrigger(el) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, ease: 'expo.out', scrollTrigger: revealTrigger(el) })
  const grid = el.querySelector('[data-honeycomb]')!
  gsap.from('[data-tile]', {
    scale: 0.6,
    autoAlpha: 0,
    duration: 0.8,
    ease: 'back.out(1.7)',
    stagger: { each: 0.035, from: 'center', grid: 'auto' },
    scrollTrigger: revealTrigger(grid, 'top 75%'),
  })
  gsap.from('[data-tile-label]', { autoAlpha: 0, y: 12, scale: 0.9, duration: 0.7, delay: 0.7, stagger: 0.25, ease: 'back.out(2)', scrollTrigger: revealTrigger(grid, 'top 75%') })
  el.querySelectorAll('[data-tile-label]').forEach((label, i) => {
    gsap.to(label, { y: -6, duration: 2.6 + i * 0.4, repeat: -1, yoyo: true, ease: 'sine.inOut', delay: 1.6 })
  })
  gsap.from('[data-trust-rule]', { scaleY: 0, transformOrigin: 'top', duration: 1, stagger: 0.12, scrollTrigger: revealTrigger(el.querySelector('[data-trust-stats]')!) })
})
</script>

<template>
  <section ref="root" aria-labelledby="trust-title" class="pt-20 lg:pt-34">
    <LContainer>
      <div class="text-center">
        <LEyebrow>لكل الفريق</LEyebrow>
        <LSplitHeading id="trust-title" :lines="['نظام واحد لكل فريقك.']" class="mt-3 text-h2 text-ink" />
        <p data-trust-sub class="mt-3 text-base text-ink-soft">كل دور يفتح على شاشته وصلاحياته — من غير ما حد يتوه.</p>
      </div>

      <div data-honeycomb class="mt-14 overflow-x-clip py-2 lg:mt-20">
        <RoleHoneycomb />
      </div>

      <ul data-trust-stats class="mt-14 grid border-b border-ink/10 sm:grid-cols-3 lg:mt-14">
        <li v-for="(stat, i) in TEAM_STATS" :key="stat.label" class="relative flex items-center justify-center gap-4 py-8 sm:py-11">
          <span v-if="i > 0" data-trust-rule class="absolute inset-y-0 inset-s-0 hidden w-px bg-ink/10 sm:block" />
          <span v-if="i > 0" class="absolute inset-x-0 top-0 h-px bg-ink/10 sm:hidden" />
          <p class="order-2 text-base text-ink-soft">{{ stat.label }}</p>
          <p class="order-1 font-display text-5xl font-medium text-ink">
            <LStatValue :stat="stat" />
          </p>
        </li>
      </ul>
    </LContainer>
  </section>
</template>
