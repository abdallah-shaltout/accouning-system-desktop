<script setup lang="ts">
import { FAQ } from '~/data/faq'

/** Below-the-fold sections hydrate shortly before they scroll in (SSR HTML is unchanged). */
const HYDRATE = { rootMargin: '600px' }

const title = `${APP_NAME_AR} — برنامج محاسبة وكاشير للمحلات يعمل بدون إنترنت`
const description =
  'برنامج محاسبة سطح مكتب للمحلات: فواتير ضريبية، كاشير POS، مخزون وجرد، و28 تقريرًا جاهزًا. يعمل بدون إنترنت وبياناتك تبقى على جهازك. حمّله مجانًا لويندوز.'

useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
  ogType: 'website',
  ogLocale: 'ar_AR',
  ogImage: '/og.png',
  ogImageAlt: `${APP_NAME_AR} — حساباتك مضبوطة، وبياناتك في محلّك`,
  twitterCard: 'summary_large_image',
})
useHead({ titleTemplate: null })

useSchemaOrg([
  defineWebPage({ '@type': ['WebPage', 'FAQPage'], name: title, description }),
  defineSoftwareApp({
    name: APP_NAME_AR,
    alternateName: APP_NAME_EN,
    description: DEFINITION_AR,
    applicationCategory: 'BusinessApplication',
    applicationSubCategory: 'Accounting software',
    operatingSystem: SUPPORTED_OS,
    inLanguage: 'ar',
    offers: { price: 0, priceCurrency: 'EGP', description: 'نسخة تجريبية مجانية' },
    featureList: [
      'فواتير ضريبية بضريبة القيمة المضافة',
      'نقطة بيع POS بالباركود',
      'إدارة مخزون وجرد دوري',
      'مشتريات وموردون',
      '28 تقريرًا ماليًا جاهزًا',
      'تعدد الفروع والصلاحيات',
      'يعمل دون اتصال بالإنترنت',
      'نسخ احتياطي مشفّر',
    ],
  }),
  ...FAQ.map((f) => defineQuestion({ name: f.q, acceptedAnswer: f.a })),
])
</script>

<template>
  <main id="main">
    <div class="bg-coral-500">
      <HeroSection />
      <TickerBand />
      <StatsBand />
      <LazyProductCards :hydrate-on-visible="HYDRATE" class="cv-auto" />
    </div>
    <div class="cv-auto bg-linear-to-b from-coral-500 to-forest-900 px-2.5">
      <LazyFeatureAccordion :hydrate-on-visible="HYDRATE" />
    </div>
    <LazyRulesSection :hydrate-on-visible="HYDRATE" class="cv-auto" />
    <div class="px-2.5">
      <div class="overflow-hidden rounded-panel bg-white">
        <LazyTrustSection :hydrate-on-visible="HYDRATE" class="cv-auto bg-white" />
        <LazyTeamSection :hydrate-on-visible="HYDRATE" class="cv-auto bg-white" />
        <LazyBusinessTypes :hydrate-on-visible="HYDRATE" class="cv-auto bg-white" />
        <LazyFaqSection :hydrate-on-visible="HYDRATE" class="cv-auto bg-white" />
        <LazyBlogSection :hydrate-on-visible="HYDRATE" class="cv-auto bg-white" />
      </div>
    </div>
    <LazyCtaSection :hydrate-on-visible="HYDRATE" class="cv-auto" />
  </main>
</template>
