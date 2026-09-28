<script setup lang="ts">
const { public: cfg } = useRuntimeConfig()
const root = ref<HTMLElement | null>(null)

// Entrance runs in CSS (.intro-* in main.css) so it starts at first paint; GSAP adds the idle
// float and the scroll-linked parallax once hydrated.
useLandingMotion(root, ({ gsap }) => {
  gsap.to('[data-hero-layer="sphere"]', { y: -14, duration: 7, repeat: -1, yoyo: true, ease: 'sine.inOut' })
  gsap.to('[data-hero-layer="beam"] ', { scaleX: 0.6, duration: 4, repeat: -1, yoyo: true, ease: 'sine.inOut' })

  const scroll = { trigger: root.value!, start: 'top top', end: 'bottom top', scrub: true }
  gsap.to('[data-hero-layer="sphere"] svg', { yPercent: -8, ease: 'none', scrollTrigger: scroll })
  gsap.to('[data-hero-layer="edge"]', { yPercent: -6, ease: 'none', scrollTrigger: scroll })
  gsap.to('[data-hero]', { y: 60, ease: 'none', scrollTrigger: scroll })
})
</script>

<template>
  <section ref="root" aria-labelledby="hero-title" class="relative px-2.5">
    <div
      class="intro-panel relative isolate flex min-h-160 flex-col justify-end overflow-hidden rounded-b-panel bg-hero-100 pt-32 pb-16 sm:min-h-176 lg:h-[min(62.8vw,57rem)] lg:min-h-168 lg:pb-34"
    >
      <HeroVisual />
      <LContainer class="relative">
        <div data-hero class="max-w-3xl lg:ps-7">
          <LEyebrow dot class="intro-up" style="--delay: 0s">برنامج المحاسبة لمحلّك</LEyebrow>
          <h1 id="hero-title" class="mt-5 text-hero text-balance text-ink">
            <span class="line-mask intro-line"><span>حساباتك مضبوطة.</span></span>
            <span class="line-mask intro-line"><span>وبياناتك في محلّك.</span></span>
          </h1>
          <p class="intro-up mt-4 max-w-160 text-lead text-ink-soft text-pretty" style="--delay: 0.3s">
            فواتير وكاشير ومخزون وتقارير على جهازك — تشتغل من غير إنترنت، وبياناتك لا تغادر المحل.
          </p>
          <div class="intro-pop mt-8 flex flex-wrap items-center gap-x-5 gap-y-3" style="--delay: 0.4s">
            <LPillButton :to="cfg.downloadUrl" size="lg">حمّل ايكوال لويندوز</LPillButton>
            <span class="text-sm text-ink-soft">مجانًا للتجربة · بدون اشتراك شهري</span>
          </div>
        </div>
      </LContainer>
    </div>
  </section>
</template>
