import type gsapType from 'gsap'
import type { ScrollTrigger as ScrollTriggerType } from 'gsap/ScrollTrigger'
import type LenisType from 'lenis'

export interface Motion {
  gsap: typeof gsapType
  ScrollTrigger: typeof ScrollTriggerType
  lenis: LenisType
}

/** Fixed nav height + breathing room; anchor jumps land below it (matches scroll-padding-top). */
const ANCHOR_OFFSET = -96

/**
 * GSAP and Lenis are loaded as their own chunk after the app is ready, so they never compete with
 * the first paint (first-paint motion is CSS; see .intro-* in main.css). Sections await `$motion`.
 *
 * Lenis smooths the page scroll and is driven by GSAP's ticker, so ScrollTrigger reads the same
 * frame Lenis writes. `respectReducedMotion: false`: motion always runs (CLAUDE.md rule 20).
 */
export default defineNuxtPlugin((nuxtApp) => {
  const motion = new Promise<Motion>((resolve) => {
    onNuxtReady(async () => {
      const [{ default: gsap }, { ScrollTrigger }, { default: Lenis }] = await Promise.all([
        import('gsap'),
        import('gsap/ScrollTrigger'),
        import('lenis'),
      ])
      gsap.registerPlugin(ScrollTrigger)
      gsap.defaults({ ease: 'power3.out', duration: 0.9 })

      const lenis = new Lenis({
        lerp: 0.1,
        wheelMultiplier: 1,
        anchors: { offset: ANCHOR_OFFSET },
        respectReducedMotion: false,
      })
      lenis.on('scroll', ScrollTrigger.update)
      gsap.ticker.add((time) => lenis.raf(time * 1000))
      gsap.ticker.lagSmoothing(0)

      // Sections hydrate lazily, so triggers are created out of page order; the accordion's pin adds
      // scroll distance that every trigger below it must include. Sort by position, then measure.
      const refresh = () => {
        ScrollTrigger.sort()
        ScrollTrigger.refresh()
      }
      document.fonts?.ready.then(refresh)
      if (document.readyState === 'complete') requestAnimationFrame(refresh)
      else window.addEventListener('load', refresh, { once: true })
      nuxtApp.hook('page:finish', () => {
        lenis.resize()
        requestAnimationFrame(refresh)
      })

      // Sections use content-visibility:auto, so the page height changes as they render in;
      // trigger positions must follow.
      let pending: ReturnType<typeof setTimeout> | undefined
      let lastHeight = 0
      new ResizeObserver(([entry]) => {
        const h = Math.round(entry!.contentRect.height)
        if (h === lastHeight) return
        lastHeight = h
        clearTimeout(pending)
        pending = setTimeout(refresh, 150)
      }).observe(document.body)

      resolve({ gsap, ScrollTrigger, lenis })
    })
  })

  return { provide: { motion } }
})
