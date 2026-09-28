<script setup lang="ts">
import type { Stat } from '~/data/stats'

/** Final value is in the SSR markup (crawlable); the count-up only runs client-side, once. */
const props = defineProps<{ stat: Stat; affixClass?: string }>()

const root = ref<HTMLElement | null>(null)
const valueEl = ref<HTMLElement | null>(null)

useLandingMotion(root, ({ gsap }) => {
  const el = valueEl.value
  if (!el) return
  const counter = { n: 0 }
  el.textContent = '0'
  gsap.to(counter, {
    n: props.stat.value,
    duration: 1.6,
    ease: 'power2.out',
    scrollTrigger: { trigger: root.value!, start: 'top 85%', once: true },
    onUpdate: () => { el.textContent = String(Math.round(counter.n)) },
  })
})
</script>

<template>
  <span ref="root" class="num inline-flex items-baseline">
    <span v-if="stat.prefix" :class="affixClass">{{ stat.prefix }}</span>
    <span ref="valueEl">{{ stat.value }}</span>
    <span v-if="stat.suffix" :class="affixClass">{{ stat.suffix }}</span>
  </span>
</template>
