import tailwindcss from '@tailwindcss/vite'

const siteUrl = process.env.NUXT_PUBLIC_SITE_URL ?? 'https://equal-app.com'

export default defineNuxtConfig({
  compatibilityDate: '2026-09-01',
  devtools: { enabled: false },
  modules: ['@nuxt/image', '@nuxt/fonts', '@nuxtjs/seo'],
  css: ['~/assets/css/main.css'],
  vite: { plugins: [tailwindcss()] },

  app: {
    head: {
      htmlAttrs: { lang: 'ar', dir: 'rtl' },
      meta: [{ name: 'theme-color', content: '#F4553C' }],
      link: [{ rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' }],
    },
  },

  fonts: {
    families: [
      { name: 'Alexandria', provider: 'google', weights: [400, 500, 600], subsets: ['arabic', 'latin'] },
      { name: 'IBM Plex Sans Arabic', provider: 'google', weights: [400, 500], subsets: ['arabic', 'latin'] },
    ],
    defaults: { preload: true },
  },

  image: { quality: 80, format: ['webp'] },

  site: {
    url: siteUrl,
    name: 'ايكوال المحاسبي',
    description:
      'برنامج محاسبة سطح مكتب للمحلات — فواتير، كاشير، مخزون وتقارير. يعمل بدون إنترنت وبياناتك تبقى على جهازك.',
    defaultLocale: 'ar',
  },

  ogImage: { enabled: false },
  linkChecker: { enabled: false },
  schemaOrg: {
    identity: {
      type: 'Organization',
      name: 'ايكوال المحاسبي',
      alternateName: 'Equal Accounting',
      url: siteUrl,
      logo: '/logo.png',
    },
  },
  sitemap: { zeroRuntime: true },

  runtimeConfig: {
    public: {
      siteUrl,
      downloadUrl: '#download',
      contactWhatsApp: '',
      contactEmail: 'hello@equal-app.com',
    },
  },

  nitro: { prerender: { crawlLinks: true, routes: ['/', '/llms.txt'] } },
})
