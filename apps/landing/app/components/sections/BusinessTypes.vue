<script setup lang="ts">
import { Store } from 'lucide-vue-next'
import { BUSINESS_TYPES } from '~/data/businessTypes'

const AUTO_S = 8
const openKey = ref(BUSINESS_TYPES[0]!.key)
const root = ref<HTMLElement | null>(null)
let progress: gsap.core.Tween | undefined
let run: (() => void) | undefined
let visible = false

/** Auto-advance never stops: a click opens that type and restarts the cycle from it. */
function open(key: string) {
  openKey.value = key
  progress?.kill()
  if (visible) nextTick(() => run?.())
}

useLandingMotion(root, ({ gsap, ScrollTrigger }) => {
  const el = root.value!
  gsap.from('[data-reveal="eyebrow"]', { autoAlpha: 0, y: 16, scrollTrigger: revealTrigger(el) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, stagger: 0.12, ease: 'expo.out', scrollTrigger: revealTrigger(el) })
  gsap.from('[data-type-row]', { autoAlpha: 0, y: 40, duration: 1, stagger: 0.1, ease: 'expo.out', scrollTrigger: revealTrigger(el.querySelector('[data-types]')!) })

  // Auto-advance while visible; the bar under the scenario shows the time left.
  run = () => {
    const bar = el.querySelector<HTMLElement>(`[data-progress="${openKey.value}"]`)
    if (!bar) return
    progress = gsap.fromTo(bar, { scaleX: 0 }, {
      scaleX: 1,
      duration: AUTO_S,
      ease: 'none',
      onComplete: () => {
        const i = BUSINESS_TYPES.findIndex((t) => t.key === openKey.value)
        openKey.value = BUSINESS_TYPES[(i + 1) % BUSINESS_TYPES.length]!.key
        nextTick(() => run?.())
      },
    })
  }
  // Only the off-screen time is skipped: the cycle resumes where it was when the list returns.
  ScrollTrigger.create({
    trigger: el.querySelector('[data-types]')!,
    start: 'top 70%',
    end: 'bottom 30%',
    onToggle: (s) => {
      visible = s.isActive
      if (!s.isActive) progress?.pause()
      else if (progress?.paused()) progress.resume()
      else run?.()
    },
  })
})
</script>

<template>
  <section id="types" ref="root" aria-labelledby="types-title" class="pt-28 lg:pt-51">
    <LContainer>
      <LEyebrow>لكل نشاط</LEyebrow>
      <LSplitHeading id="types-title" :lines="['يشتغل مع كل', 'أنواع المحلات.']" class="mt-3 text-h2 text-ink" />

      <ul data-types class="mt-12 lg:mt-19">
        <li v-for="(t, i) in BUSINESS_TYPES" :key="t.key" data-type-row>
          <div v-if="i > 0" class="my-5.5 h-px bg-ink/10" />
          <LAccordionBar :id="`type-${t.key}`" :title="t.name" :icon="t.icon" :open="openKey === t.key" @toggle="open(t.key)">
            <p class="max-w-120 font-display text-xl leading-relaxed text-ink text-pretty lg:text-[1.375rem]">«{{ t.scenario }}»</p>
            <span class="mt-5 block h-0.5 w-35 overflow-hidden bg-ink/10">
              <span :data-progress="t.key" class="block h-full origin-right scale-x-0 bg-ink" />
            </span>
            <p class="mt-7 flex items-center gap-3">
              <span class="grid size-8 place-items-center rounded-full bg-cream text-coral-600"><Store class="size-4" /></span>
              <span class="font-display text-lg text-ink">مناسب لـ</span>
              <span class="text-base text-ink-soft">{{ t.fits }}</span>
            </p>
          </LAccordionBar>
        </li>
      </ul>
    </LContainer>
  </section>
</template>
