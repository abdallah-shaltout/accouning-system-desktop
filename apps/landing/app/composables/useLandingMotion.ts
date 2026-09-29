import type { Ref } from 'vue'
import type { Motion } from '~/plugins/gsap.client'

export interface MotionContext extends Motion {
  /** RTL sign: "move toward the reading direction" is `x: 40 * dir`. */
  dir: -1
  /** The section root, for scoping selectors and triggers. */
  root: HTMLElement
}

/**
 * Every section animates through this: selectors are scoped to `root`, and every tween and
 * ScrollTrigger is reverted on unmount (route changes never leave stale triggers). GSAP arrives
 * asynchronously (see plugins/gsap.client.ts), so setup runs once it has loaded.
 */
export function useLandingMotion(root: Ref<HTMLElement | null>, setup: (ctx: MotionContext) => void | (() => void)) {
  let ctx: gsap.Context | undefined
  let alive = true
  onMounted(async () => {
    const { $motion } = useNuxtApp()
    const { gsap, ScrollTrigger, lenis } = await ($motion as Promise<Motion>)
    const el = root.value
    if (!alive || !el) return
    ctx = gsap.context(() => setup({ gsap, ScrollTrigger, lenis, dir: -1, root: el }), el)
  })
  onBeforeUnmount(() => {
    alive = false
    ctx?.revert()
  })
}

/** Standard once-only reveal trigger used across sections. */
export const revealTrigger = (trigger: Element | string, start = 'top 80%') => ({
  trigger,
  start,
  once: true,
})
