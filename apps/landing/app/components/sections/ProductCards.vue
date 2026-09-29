<script setup lang="ts">
import { PRODUCT_CARDS } from '~/data/features'
import { SHOTS } from '~/data/screens'

const root = ref<HTMLElement | null>(null)

useLandingMotion(root, ({ gsap, ScrollTrigger }) => {
  gsap.from('[data-reveal="eyebrow"]', { autoAlpha: 0, y: 16, scrollTrigger: revealTrigger(root.value!) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, stagger: 0.12, ease: 'expo.out', scrollTrigger: revealTrigger(root.value!) })
  gsap.set('[data-product-card]', { autoAlpha: 0, y: 56 })
  ScrollTrigger.batch('[data-product-card]', {
    start: 'top 88%',
    once: true,
    onEnter: (els) => gsap.to(els, { autoAlpha: 1, y: 0, duration: 1.1, stagger: 0.12, ease: 'expo.out' }),
  })
  gsap.from('[data-product-rule]', { scaleY: 0, transformOrigin: 'top', duration: 1.2, stagger: 0.1, scrollTrigger: revealTrigger(root.value!.querySelector('[data-product-grid]')!) })
  gsap.from('[data-product-shot]', { y: 32, scale: 0.97, duration: 1.4, stagger: 0.12, delay: 0.2, ease: 'expo.out', scrollTrigger: revealTrigger(root.value!.querySelector('[data-product-grid]')!) })
})
</script>

<template>
  <section id="features" ref="root" aria-labelledby="product-title" class="pt-28 pb-12 lg:pt-61">
    <LContainer>
      <div class="text-center">
        <LEyebrow tone="white">نظام واحد لكل المحل</LEyebrow>
        <LSplitHeading
          id="product-title"
          :lines="['كل اللي فلوسك محتاجاه.', 'ومن غير أي تعقيد.']"
          :dim="[1]"
          dim-class="opacity-50"
          class="mt-4 text-h2 text-white"
        />
      </div>

      <div data-product-grid class="mt-14 grid gap-6 md:grid-cols-3 md:gap-11 lg:mt-18">
        <article
          v-for="(card, i) in PRODUCT_CARDS"
          :key="card.shot"
          data-product-card
          class="group relative flex flex-col"
        >
          <span v-if="i > 0" data-product-rule class="absolute inset-y-0 -inset-s-5.5 hidden w-px bg-white/25 md:block" />
          <!-- The media box is the shot's own ratio (all three crops share it), so the cards line up. -->
          <div class="overflow-hidden rounded-t-card bg-white/15 p-3 sm:p-4 lg:p-5">
            <div class="transition-transform duration-500 ease-out-expo group-hover:-translate-y-1">
              <LAppShot data-product-shot :shot="SHOTS[card.shot]" sizes="xs:92vw md:31vw lg:420px" />
            </div>
          </div>
          <div class="flex-1 rounded-b-card bg-cream px-6 pt-6 pb-7">
            <h3 class="text-h3 text-ink">{{ card.title }}</h3>
            <p class="mt-2 text-base leading-relaxed text-ink-soft text-pretty">{{ card.body }}</p>
          </div>
        </article>
      </div>
    </LContainer>
  </section>
</template>
