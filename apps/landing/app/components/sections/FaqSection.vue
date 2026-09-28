<script setup lang="ts">
import { FAQ } from '~/data/faq'

const openIndex = ref(0)
const root = ref<HTMLElement | null>(null)

useLandingMotion(root, ({ gsap }) => {
  const el = root.value!
  gsap.from('[data-reveal="eyebrow"]', { autoAlpha: 0, y: 16, scrollTrigger: revealTrigger(el) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, ease: 'expo.out', scrollTrigger: revealTrigger(el) })
  gsap.from('[data-faq-row]', { autoAlpha: 0, y: 32, duration: 0.9, stagger: 0.07, ease: 'expo.out', scrollTrigger: revealTrigger(el.querySelector('[data-faq]')!) })
})
</script>

<template>
  <section id="faq" ref="root" aria-labelledby="faq-title" class="pt-28 lg:pt-44">
    <LContainer>
      <LEyebrow>الأسئلة الشائعة</LEyebrow>
      <LSplitHeading id="faq-title" :lines="['أسئلة بتتسأل كتير.']" class="mt-3 text-h2 text-ink" />

      <ul data-faq class="mt-12 lg:mt-16">
        <li v-for="(item, i) in FAQ" :key="item.q" data-faq-row>
          <div v-if="i > 0" class="my-3.5 h-px bg-ink/10" />
          <LAccordionBar :id="`faq-${i}`" :title="item.q" :open="openIndex === i" @toggle="openIndex = openIndex === i ? -1 : i">
            <p class="max-w-160 text-lg leading-relaxed text-ink-soft">{{ item.a }}</p>
          </LAccordionBar>
        </li>
      </ul>
    </LContainer>
  </section>
</template>
