<script setup lang="ts">
import { TEAM_FEATURES } from '~/data/features'
import { SHOTS } from '~/data/screens'

const root = ref<HTMLElement | null>(null)

useLandingMotion(root, ({ gsap, ScrollTrigger }) => {
  const el = root.value!
  gsap.from('[data-reveal="eyebrow"]', { autoAlpha: 0, y: 16, scrollTrigger: revealTrigger(el) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, stagger: 0.12, ease: 'expo.out', scrollTrigger: revealTrigger(el) })
  const panels = el.querySelector('[data-team-panels]')!
  gsap.from('[data-team-panel]', { autoAlpha: 0, y: 64, duration: 1.2, stagger: 0.14, ease: 'expo.out', scrollTrigger: revealTrigger(panels) })
  gsap.to('[data-team-float]', { y: -10, duration: 3.2, repeat: -1, yoyo: true, ease: 'sine.inOut', stagger: 0.8 })
  gsap.to('[data-team-panel] > [data-parallax]', {
    yPercent: -6,
    ease: 'none',
    scrollTrigger: { trigger: panels, start: 'top bottom', end: 'bottom top', scrub: 0.8 },
  })
  gsap.set('[data-team-feature]', { autoAlpha: 0, y: 18 })
  ScrollTrigger.batch('[data-team-feature]', {
    start: 'top 90%',
    once: true,
    onEnter: (els) => gsap.to(els, { autoAlpha: 1, y: 0, duration: 0.8, stagger: 0.08 }),
  })
})
</script>

<template>
  <section ref="root" aria-labelledby="team-title" class="pt-28 lg:pt-60">
    <LContainer>
      <LEyebrow>مكان واحد. كل الفريق.</LEyebrow>
      <LSplitHeading
        id="team-title"
        :lines="['كل اللي فريقك محتاجه', 'عشان يشتغل على دفاتر واحدة.']"
        class="mt-3 max-w-232 text-h2 text-ink"
      />

      <div data-team-panels class="mt-12 grid gap-6 md:grid-cols-2 md:gap-11 lg:mt-20">
        <div data-team-panel class="panel-grad relative grid min-h-120 place-items-center overflow-hidden rounded-bar px-6 py-14 lg:h-194">
          <div data-parallax class="w-full">
            <div data-team-float><LAppShot :shot="SHOTS.analytics" :bleed="1.3" sizes="xs:130vw md:65vw lg:860px" /></div>
          </div>
        </div>
        <div data-team-panel class="relative grid min-h-120 place-items-center overflow-hidden rounded-bar bg-panel-100 px-6 py-14 lg:h-194">
          <span class="absolute inset-y-0 -inset-s-5.5 hidden w-px bg-ink/10 md:block" />
          <div data-parallax class="w-full">
            <div data-team-float><LAppShot :shot="SHOTS.roles" :bleed="1.3" sizes="xs:130vw md:65vw lg:860px" /></div>
          </div>
        </div>
      </div>

      <ul class="mt-24 grid border-ink/10 sm:grid-cols-6 lg:mt-39" aria-label="مزايا للفريق">
        <li
          v-for="(f, i) in TEAM_FEATURES"
          :key="f.label"
          data-team-feature
          class="flex h-24 items-center justify-center gap-3 border-ink/10 text-center font-display text-lg text-ink sm:h-30 lg:text-xl"
          :class="[
            i < 3 ? 'border-b sm:col-span-2' : 'sm:col-span-3',
            i === 1 || i === 2 ? 'sm:border-s' : '',
            i === 4 ? 'sm:border-s' : '',
            i === 3 ? 'border-b sm:border-b-0' : '',
          ]"
        >
          <component :is="f.icon" class="size-5.5 shrink-0" :stroke-width="1.6" />
          {{ f.label }}
        </li>
      </ul>
    </LContainer>
  </section>
</template>

<style scoped>
.panel-grad { background: linear-gradient(180deg, var(--color-panel-50), var(--color-panel-100) 55%, var(--color-panel-300)); }
</style>
