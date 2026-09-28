<script setup lang="ts">
import { FOOTER_COLUMNS, LEGAL_LINKS } from '~/data/nav'

const root = ref<HTMLElement | null>(null)
const year = 2026

useLandingMotion(root, ({ gsap }) => {
  const el = root.value!
  gsap.from('[data-line]', { yPercent: 110, duration: 1.2, stagger: 0.12, ease: 'expo.out', scrollTrigger: revealTrigger(el, 'top 90%') })
  gsap.from('[data-footer-col]', { autoAlpha: 0, y: 20, duration: 0.8, stagger: 0.08, scrollTrigger: revealTrigger(el, 'top 85%') })
})

const isExternal = (to: string) => /^(https?:|mailto:)/.test(to)
</script>

<template>
  <footer ref="root" class="pb-8 text-white">
    <LContainer>
      <div class="grid gap-14 lg:grid-cols-[1fr_auto]">
        <div>
          <NuxtLink to="/" class="inline-flex text-2xl" :aria-label="APP_NAME_AR">
            <LLogo tone="white" wordmark />
          </NuxtLink>
          <LSplitHeading as="p" :lines="['محاسبة،', 'ببساطة.']" class="mt-10 font-display text-footer font-medium" />
          <p class="mt-6 max-w-96 text-sm leading-relaxed text-white/55">
            {{ DEFINITION_AR }} فواتير، كاشير، مخزون وتقارير — {{ APP_NAME_EN }}.
          </p>
        </div>

        <nav aria-label="روابط الموقع" class="grid grid-cols-2 gap-x-16 gap-y-16 sm:gap-x-24 lg:gap-y-24">
          <div v-for="col in FOOTER_COLUMNS" :key="col.title" data-footer-col>
            <p class="text-eyebrow font-semibold text-white">{{ col.title }}</p>
            <ul class="mt-5 space-y-3.5">
              <li v-for="link in col.links" :key="link.label">
                <a v-if="isExternal(link.to)" :href="link.to" class="text-base text-white/75 transition-colors hover:text-white">{{ link.label }}</a>
                <NuxtLink v-else :to="link.to" class="text-base text-white/75 transition-colors hover:text-white">{{ link.label }}</NuxtLink>
              </li>
            </ul>
          </div>
        </nav>
      </div>

      <div class="mt-20 flex flex-col-reverse items-start justify-between gap-6 border-t border-white/15 pt-6 sm:flex-row sm:items-center lg:mt-32">
        <p class="text-sm text-white/55">© <span class="num">{{ year }}</span> {{ APP_NAME_AR }}</p>
        <ul class="flex items-center gap-10">
          <li v-for="link in LEGAL_LINKS" :key="link.to">
            <NuxtLink :to="link.to" class="text-eyebrow text-white/75 transition-colors hover:text-white">{{ link.label }}</NuxtLink>
          </li>
        </ul>
      </div>
    </LContainer>
  </footer>
</template>
