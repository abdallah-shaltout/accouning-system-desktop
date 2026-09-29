import type { RouterConfig } from '@nuxt/schema'

/** Fixed nav height + breathing room (matches scroll-padding-top and Lenis `anchors.offset`). */
const NAV_OFFSET = 96

/**
 * Same-page `#section` links are animated by Lenis (`anchors` in plugins/gsap.client.ts), so the
 * router must not jump there as well. Other navigations wait for the new page to render first.
 */
export default {
  scrollBehavior(to, from, saved) {
    if (to.path === from.path && to.hash) return false
    const nuxtApp = useNuxtApp()
    return new Promise((resolve) => {
      nuxtApp.hooks.hookOnce('page:finish', () => {
        requestAnimationFrame(() => resolve(saved ?? (to.hash ? { el: to.hash, top: NAV_OFFSET } : { top: 0 })))
      })
    })
  },
} satisfies RouterConfig
