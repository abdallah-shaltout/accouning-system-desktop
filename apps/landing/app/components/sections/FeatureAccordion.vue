<script setup lang="ts">
import { Boxes, ChartColumn, Receipt, ScanBarcode, Truck } from 'lucide-vue-next'
import { ACCORDION_FEATURES, type AccordionFeature } from '~/data/features'

const ICONS = { pos: ScanBarcode, invoices: Receipt, stock: Boxes, purchases: Truck, reports: ChartColumn }
/** Scroll distance (in viewport heights) each feature stays active while the list is pinned. */
const STEP_VH = 0.55
/** Below lg the list is too tall to pin, so it auto-advances instead. */
const AUTO_MS = 6000
const COUNT = ACCORDION_FEATURES.length

const active = ref<AccordionFeature['key']>('pos')
const root = ref<HTMLElement | null>(null)
let inView = false
let timer: ReturnType<typeof setInterval> | undefined
/** Set while the pinned (lg+) layout is live: scrolls to the part of the pin that shows feature `i`. */
let jumpTo: ((i: number) => void) | undefined

function select(key: AccordionFeature['key']) {
  const i = ACCORDION_FEATURES.findIndex((f) => f.key === key)
  if (jumpTo) return jumpTo(i)
  // Auto-advance never stops: a click jumps to that feature and restarts the cycle from it.
  active.value = key
  clearInterval(timer)
  timer = setInterval(advance, AUTO_MS)
}

function advance() {
  if (!inView) return
  const i = ACCORDION_FEATURES.findIndex((f) => f.key === active.value)
  active.value = ACCORDION_FEATURES[(i + 1) % COUNT]!.key
}

useLandingMotion(root, ({ gsap, ScrollTrigger, lenis }) => {
  const el = root.value!
  const pinEl = el.querySelector<HTMLElement>('[data-acc-pin]')!
  gsap.from('[data-reveal="eyebrow"]', { autoAlpha: 0, y: 16, scrollTrigger: revealTrigger(el) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, ease: 'expo.out', scrollTrigger: revealTrigger(el) })
  gsap.from('[data-acc-row]', { autoAlpha: 0, x: 32 * -1, duration: 0.9, stagger: 0.08, scrollTrigger: revealTrigger(pinEl) })
  gsap.from('[data-acc-visual]', { autoAlpha: 0, y: 60, duration: 1.2, ease: 'expo.out', scrollTrigger: revealTrigger(pinEl) })

  const mm = gsap.matchMedia()

  // lg+: the list and the screen stay pinned; each short scroll hands "active" to the next feature.
  mm.add('(min-width: 1024px)', () => {
    const distance = () => COUNT * STEP_VH * window.innerHeight
    const pin = ScrollTrigger.create({
      trigger: pinEl,
      pin: true,
      start: 'center center',
      end: () => `+=${distance()}`,
      invalidateOnRefresh: true,
      anticipatePin: 1,
      onUpdate: (s) => {
        const key = ACCORDION_FEATURES[Math.min(COUNT - 1, Math.floor(s.progress * COUNT))]!.key
        if (key !== active.value) active.value = key
      },
    })
    gsap.fromTo(
      '[data-acc-fill]',
      { scaleY: 0 },
      { scaleY: 1, ease: 'none', scrollTrigger: { trigger: pinEl, start: 'center center', end: () => `+=${distance()}`, scrub: true, invalidateOnRefresh: true } },
    )
    gsap.to('[data-scene-card]', {
      yPercent: -5,
      ease: 'none',
      scrollTrigger: { trigger: pinEl, start: 'center center', end: () => `+=${distance()}`, scrub: 0.8, invalidateOnRefresh: true },
    })
    jumpTo = (i) => lenis.scrollTo(pin.start + ((i + 0.5) / COUNT) * (pin.end - pin.start))
    return () => {
      jumpTo = undefined
    }
  })

  // Below lg: no pin; auto-advance while the section is on screen.
  mm.add('(max-width: 1023.98px)', () => {
    ScrollTrigger.create({ trigger: el, start: 'top 60%', end: 'bottom 40%', onToggle: (s) => { inView = s.isActive } })
    timer = setInterval(advance, AUTO_MS)
    return () => clearInterval(timer)
  })

  return () => mm.revert()
})
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <section id="screens" ref="root" aria-labelledby="accordion-title" class="rounded-panel bg-white pt-20 pb-16 lg:pt-27 lg:pb-24">
    <LContainer>
      <LEyebrow>مبني لشغل حقيقي</LEyebrow>
      <LSplitHeading id="accordion-title" :lines="['محاسبة تلحق شغلك.']" class="mt-3 text-h2 text-ink" />

      <div data-acc-pin class="mt-10 grid items-center gap-12 lg:grid-cols-[minmax(0,26rem)_1fr] lg:gap-16">
        <div class="relative">
          <!-- Scroll progress through the pinned list (lg+), filling downward on the start edge. -->
          <span class="absolute inset-y-2 -inset-s-6 hidden w-px bg-ink/10 lg:block" aria-hidden="true">
            <span data-acc-fill class="block h-full w-full origin-top scale-y-0 bg-coral-500" />
          </span>
          <ul data-acc-list>
            <li v-for="f in ACCORDION_FEATURES" :key="f.key" data-acc-row class="border-b border-ink/10 last:border-b-0">
              <h3>
                <button
                  :id="`acc-${f.key}`"
                  type="button"
                  class="group flex w-full items-center gap-4 pt-7 text-start transition-[padding] duration-700 ease-out-expo"
                  :class="active === f.key ? 'pb-3' : 'pb-7'"
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
                  <p class="max-w-80 ps-10 pb-8 text-base leading-relaxed text-ink-soft">{{ f.body }}</p>
                </div>
              </div>
            </li>
          </ul>
        </div>

        <div data-acc-visual class="w-full">
          <FeatureScene :active="active" :features="ACCORDION_FEATURES" />
        </div>
      </div>
    </LContainer>
  </section>
</template>
