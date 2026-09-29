<script setup lang="ts">
import { Menu, X } from 'lucide-vue-next'
import { NAV_CTA, NAV_LINKS } from '~/data/nav'
import type { Motion } from '~/plugins/gsap.client'

const { public: cfg } = useRuntimeConfig()
const open = ref(false)
const scrolled = ref(false)
const root = ref<HTMLElement | null>(null)

useLandingMotion(root, ({ ScrollTrigger }) => {
  ScrollTrigger.create({ start: 40, end: 'max', onToggle: (self) => { scrolled.value = self.isActive } })
})

const route = useRoute()
watch(() => route.fullPath, () => { open.value = false })
watch(open, async (v) => {
  if (!import.meta.client) return
  document.documentElement.style.overflow = v ? 'hidden' : ''
  const { lenis } = await (useNuxtApp().$motion as Promise<Motion>)
  if (v) lenis.stop()
  else lenis.start()
})
</script>

<template>
  <header ref="root" class="pointer-events-none fixed inset-x-0 top-3 z-40 sm:top-6">
    <LContainer>
      <nav
        data-nav-bar
        aria-label="التنقل الرئيسي"
        class="intro-nav pointer-events-auto grid h-13 lg:mx-7.5 grid-cols-[1fr_auto] items-center rounded-full bg-white/95 ps-5 pe-1.5 backdrop-blur-md transition-shadow duration-500 lg:grid-cols-[1fr_auto_1fr]"
        :class="scrolled ? 'shadow-soft ring-1 ring-ink/5' : 'shadow-[0_1px_0_rgb(8_36_34/0.04)]'"
      >
        <NuxtLink to="/" class="justify-self-start text-lg" :aria-label="APP_NAME_AR">
          <LLogo />
        </NuxtLink>

        <ul class="hidden items-center gap-7 lg:flex">
          <li v-for="link in NAV_LINKS" :key="link.to">
            <NuxtLink :to="link.to" class="text-eyebrow text-ink transition-colors hover:text-coral-500">{{ link.label }}</NuxtLink>
          </li>
        </ul>

        <div class="flex items-center gap-2 justify-self-end">
          <LPillButton :to="cfg.downloadUrl" size="sm" class="h-10! px-6!">{{ NAV_CTA }}</LPillButton>
          <button
            type="button"
            class="grid size-10 place-items-center rounded-full text-ink hover:bg-panel-100 lg:hidden"
            :aria-expanded="open"
            aria-controls="mobile-menu"
            aria-label="القائمة"
            @click="open = !open"
          >
            <X v-if="open" class="size-5" />
            <Menu v-else class="size-5" />
          </button>
        </div>
      </nav>
    </LContainer>

    <Transition
      enter-active-class="transition duration-500 ease-out-expo"
      enter-from-class="opacity-0 -translate-y-4"
      leave-active-class="transition duration-300"
      leave-to-class="opacity-0"
    >
      <div
        v-if="open"
        id="mobile-menu"
        class="pointer-events-auto fixed inset-x-3 top-19 bottom-3 flex flex-col rounded-panel bg-forest-900 p-8 text-white sm:inset-x-6 sm:top-24 lg:hidden"
      >
        <ul class="flex flex-col gap-1">
          <li v-for="link in NAV_LINKS" :key="link.to">
            <NuxtLink :to="link.to" class="block border-b border-white/10 py-4 font-display text-2xl" @click="open = false">{{ link.label }}</NuxtLink>
          </li>
        </ul>
        <LPillButton :to="cfg.downloadUrl" size="lg" class="mt-auto w-full" @click="open = false">{{ NAV_CTA }}</LPillButton>
      </div>
    </Transition>
  </header>
</template>
