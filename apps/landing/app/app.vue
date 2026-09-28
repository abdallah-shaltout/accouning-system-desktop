<script setup lang="ts">
useHead({
  titleTemplate: (t) => (t ? `${t} | ${APP_NAME_AR}` : `${APP_NAME_AR} — برنامج محاسبة وكاشير للمحلات`),
})

useSchemaOrg([
  defineWebSite({ name: APP_NAME_AR, alternateName: APP_NAME_EN, inLanguage: 'ar' }),
])

const route = useRoute()
const bare = computed(() => route.meta.bare === true)
</script>

<template>
  <div>
    <template v-if="!bare">
      <a
        href="#main"
        class="fixed inset-s-4 top-4 z-50 -translate-y-24 rounded-full bg-ink px-5 py-3 text-white transition-transform focus:translate-y-0"
      >
        تخطَّ إلى المحتوى
      </a>
      <SiteNav />
    </template>
    <NuxtPage />
    <LazySiteFooter v-if="!bare" :hydrate-on-visible="{ rootMargin: '600px' }" />
  </div>
</template>
