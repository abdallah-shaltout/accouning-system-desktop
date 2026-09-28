<script setup lang="ts">
import { RULE_MODES, type RuleMode } from '~/data/features'

const mode = ref<RuleMode['key']>('single')
const current = computed(() => RULE_MODES.find((m) => m.key === mode.value)!)
const root = ref<HTMLElement | null>(null)

useLandingMotion(root, ({ gsap }) => {
  const el = root.value!
  gsap.from('[data-reveal="eyebrow"], [data-rules-sub], [data-rules-toggle]', { autoAlpha: 0, y: 18, stagger: 0.1, scrollTrigger: revealTrigger(el) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, ease: 'expo.out', scrollTrigger: revealTrigger(el) })
  gsap.from('[data-rule-line]', { scaleX: 0, transformOrigin: 'right', duration: 1.2, stagger: 0.15, scrollTrigger: revealTrigger(el.querySelector('[data-rules-list]')!) })
  gsap.fromTo(
    '[data-stack-card]',
    { '--spread': 0 },
    { '--spread': 1, ease: 'none', scrollTrigger: { trigger: el, start: 'top 80%', end: 'bottom 30%', scrub: 1 } },
  )
  gsap.from('[data-stack]', { autoAlpha: 0, x: -80, duration: 1.4, ease: 'expo.out', scrollTrigger: revealTrigger(el, 'top 70%') })
})
</script>

<template>
  <section id="why" ref="root" aria-labelledby="rules-title" class="relative overflow-hidden bg-forest-900 pt-24 pb-24 text-white lg:pt-29 lg:pb-40">
    <div data-stack class="absolute inset-e-0 top-[38%] hidden h-[34rem] w-[46%] lg:block">
      <StackedCards />
    </div>

    <LContainer class="relative">
      <LEyebrow tone="white">سواء محل واحد أو سلسلة فروع</LEyebrow>
      <LSplitHeading id="rules-title" :lines="['دفاترك. قواعدك.']" class="mt-3 text-h2" />
      <p data-rules-sub class="mt-4 max-w-140 text-base leading-relaxed text-white/85">
        ايكوال يتشكّل على طريقة شغلك — نفس البرنامج يكبر معاك من أول وردية لحد آخر فرع.
      </p>

      <div data-rules-toggle role="tablist" aria-label="نوع النشاط" class="mt-8 flex items-center gap-2">
        <button
          v-for="m in RULE_MODES"
          :key="m.key"
          role="tab"
          type="button"
          :aria-selected="mode === m.key"
          aria-controls="rules-panel"
          class="h-10.5 rounded-full border px-4 text-base transition-colors duration-300"
          :class="mode === m.key ? 'border-white/20 bg-white/10 text-white' : 'border-transparent text-white/85 hover:text-white'"
          @click="mode = m.key"
        >{{ m.label }}</button>
      </div>

      <div id="rules-panel" role="tabpanel" data-rules-list class="mt-20 max-w-135 lg:mt-60">
        <Transition mode="out-in" enter-active-class="transition duration-500 ease-out-expo" enter-from-class="opacity-0 translate-y-4" leave-active-class="transition duration-200" leave-to-class="opacity-0">
          <ul :key="mode">
            <li v-for="(item, i) in current.items" :key="item.title" class="relative">
              <span v-if="i > 0" data-rule-line class="absolute inset-x-0 top-0 h-px bg-white/15" />
              <div class="py-10" :class="i === 0 ? 'pt-0' : ''">
                <h3 class="font-display text-h3 text-white">{{ item.title }}</h3>
                <p class="mt-2 text-eyebrow text-white/70">{{ item.body }}</p>
              </div>
            </li>
          </ul>
        </Transition>
      </div>

      <div class="mt-10 h-72 lg:hidden">
        <StackedCards />
      </div>
    </LContainer>
  </section>
</template>
