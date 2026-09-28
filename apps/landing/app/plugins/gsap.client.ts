import type gsapType from 'gsap'
import type { ScrollTrigger as ScrollTriggerType } from 'gsap/ScrollTrigger'

export interface Motion {
  gsap: typeof gsapType
  ScrollTrigger: typeof ScrollTriggerType
}

/**
 * GSAP is loaded as its own chunk after the app is ready, so it never competes with the first
 * paint (first-paint motion is CSS; see .intro-* in main.css). Sections await `$motion`.
 */
export default defineNuxtPlugin((nuxtApp) => {
  const motion = new Promise<Motion>((resolve) => {
    onNuxtReady(async () => {
      const [{ default: gsap }, { ScrollTrigger }] = await Promise.all([import('gsap'), import('gsap/ScrollTrigger')])
      gsap.registerPlugin(ScrollTrigger)
      gsap.defaults({ ease: 'power3.out', duration: 0.9 })

      const refresh = () => ScrollTrigger.refresh()
      document.fonts?.ready.then(refresh)
      if (document.readyState === 'complete') requestAnimationFrame(refresh)
      else window.addEventListener('load', refresh, { once: true })
      nuxtApp.hook('page:finish', () => {
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

      resolve({ gsap, ScrollTrigger })
    })
  })

  return { provide: { motion } }
})
