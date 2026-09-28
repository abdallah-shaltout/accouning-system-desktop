<script setup lang="ts">
const { public: cfg } = useRuntimeConfig()
const root = ref<HTMLElement | null>(null)

useLandingMotion(root, ({ gsap }) => {
  const el = root.value!
  gsap.from('[data-reveal="eyebrow"]', { autoAlpha: 0, y: 16, scrollTrigger: revealTrigger(el) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, stagger: 0.12, ease: 'expo.out', scrollTrigger: revealTrigger(el) })
  gsap.from('[data-cta-btn]', { autoAlpha: 0, scale: 0.85, duration: 0.9, delay: 0.35, ease: 'back.out(1.7)', scrollTrigger: revealTrigger(el) })
})
</script>

<template>
  <section id="download" ref="root" aria-labelledby="cta-title" class="pt-24 pb-28 text-center text-white lg:pt-39 lg:pb-58">
    <LContainer>
      <LEyebrow tone="white" class="text-sm!">ابدأ النهارده</LEyebrow>
      <LSplitHeading id="cta-title" :lines="['جاهز تمسك حسابات', 'محلّك بجد؟']" class="mt-3 text-h2" />
      <div data-cta-btn class="mt-12 flex flex-col items-center gap-4 lg:mt-19">
        <LPillButton :to="cfg.downloadUrl" size="lg">حمّل ايكوال مجانًا</LPillButton>
        <p class="text-sm text-white/55">يعمل على ويندوز <span class="num">10</span> و<span class="num">11</span> · بدون اشتراك شهري</p>
      </div>
    </LContainer>
  </section>
</template>
