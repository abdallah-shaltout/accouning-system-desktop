<script setup lang="ts">
import { Boxes, ChartColumn, Receipt, ScanBarcode, Truck } from 'lucide-vue-next'
import { ACCORDION_FEATURES, type AccordionFeature } from '~/data/features'

const ICONS = { pos: ScanBarcode, invoices: Receipt, stock: Boxes, purchases: Truck, reports: ChartColumn }
const AUTO_MS = 6000

const active = ref<AccordionFeature['key']>('pos')
const root = ref<HTMLElement | null>(null)
const paused = ref(false)
let inView = false
let timer: ReturnType<typeof setInterval> | undefined

function select(key: AccordionFeature['key']) {
  active.value = key
  paused.value = true
}

function advance() {
  if (paused.value || !inView) return
  const i = ACCORDION_FEATURES.findIndex((f) => f.key === active.value)
  active.value = ACCORDION_FEATURES[(i + 1) % ACCORDION_FEATURES.length]!.key
}

useLandingMotion(root, ({ gsap, ScrollTrigger }) => {
  const el = root.value!
  gsap.from('[data-reveal="eyebrow"]', { autoAlpha: 0, y: 16, scrollTrigger: revealTrigger(el) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, ease: 'expo.out', scrollTrigger: revealTrigger(el) })
  gsap.from('[data-acc-row]', { autoAlpha: 0, x: 32 * -1, duration: 0.9, stagger: 0.08, scrollTrigger: revealTrigger(el.querySelector('[data-acc-list]')!) })
  gsap.from('[data-acc-visual]', { autoAlpha: 0, y: 60, duration: 1.2, ease: 'expo.out', scrollTrigger: revealTrigger(el.querySelector('[data-acc-visual]')!) })
  gsap.to('[data-acc-visual] [data-scene-card]', {
    yPercent: -12,
    ease: 'none',
    scrollTrigger: { trigger: el.querySelector('[data-acc-visual]')!, start: 'top bottom', end: 'bottom top', scrub: 0.8 },
  })
  ScrollTrigger.create({ trigger: el, start: 'top 60%', end: 'bottom 40%', onToggle: (s) => { inView = s.isActive } })
  timer = setInterval(advance, AUTO_MS)
  return () => clearInterval(timer)
})
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <section id="screens" ref="root" aria-labelledby="accordion-title" class="rounded-panel bg-white pt-20 pb-16 lg:pt-27 lg:pb-24">
    <LContainer>
      <LEyebrow>مبني لشغل حقيقي</LEyebrow>
      <LSplitHeading id="accordion-title" :lines="['محاسبة تلحق شغلك.']" class="mt-3 text-h2 text-ink" />

      <div class="mt-10 grid items-start gap-12 lg:grid-cols-[minmax(0,26rem)_1fr] lg:gap-16">
        <ul data-acc-list class="lg:mt-6" @mouseleave="paused = false">
          <li v-for="f in ACCORDION_FEATURES" :key="f.key" data-acc-row class="border-b border-ink/10 last:border-b-0">
            <h3>
              <button
                :id="`acc-${f.key}`"
                type="button"
                class="group flex w-full items-center gap-4 pt-7 text-start transition-[padding] duration-700 ease-out-expo lg:pt-10"
                :class="active === f.key ? 'pb-3' : 'pb-7 lg:pb-10'"
                :aria-expanded="active === f.key"
                :aria-controls="`acc-panel-${f.key}`"
                @click="select(f.key)"
              >
                <span
                  class="grid size-6 shrink-0 place-items-center rounded-md border-[1.5px] transition-colors duration-500"
                  :class="active === f.key ? 'border-coral-500 text-coral-500' : 'border-ink-soft/70 text-ink-soft group-hover:text-ink'"
                >
                  <component :is="ICONS[f.key]" class="size-3.5" :stroke-width="2" />
                </span>
                <span
                  class="font-display text-h3 transition-colors duration-500"
                  :class="active === f.key ? 'text-coral-500' : 'text-ink-soft group-hover:text-ink'"
                >{{ f.title }}</span>
              </button>
            </h3>
            <div
              :id="`acc-panel-${f.key}`"
              role="region"
              :aria-labelledby="`acc-${f.key}`"
              class="grid transition-[grid-template-rows,opacity] duration-700 ease-out-expo"
              :class="active === f.key ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'"
            >
              <div class="overflow-hidden">
                <p class="max-w-80 ps-10 pb-10 text-base leading-relaxed text-ink-soft lg:pb-12">{{ f.body }}</p>
              </div>
            </div>
          </li>
        </ul>

        <div data-acc-visual class="w-full lg:max-w-133 lg:justify-self-end" @mouseenter="paused = true" @mouseleave="paused = false">
          <FeatureScene :active="active" :features="ACCORDION_FEATURES" />
        </div>
      </div>
    </LContainer>
  </section>
</template>
