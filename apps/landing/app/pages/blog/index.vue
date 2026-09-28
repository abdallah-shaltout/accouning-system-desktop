<script setup lang="ts">
import { BLOG_POSTS } from '~/data/blog'

const title = 'المدونة'
const description = 'مقالات عملية لصاحب المحل عن الفاتورة الضريبية، الجرد بدون إغلاق، وإدارة الحسابات بدون إنترنت.'

useSeoMeta({ title, description, ogTitle: `${title} | ${APP_NAME_AR}`, ogDescription: description, ogImage: '/og.png' })
useSchemaOrg([
  defineWebPage({ '@type': 'CollectionPage', name: title, description }),
  defineBreadcrumb({ itemListElement: [{ name: 'الرئيسية', item: '/' }, { name: title, item: '/blog' }] }),
])
</script>

<template>
  <main id="main">
    <LPagePanel>
      <LContainer>
        <LBreadcrumb :items="[{ label: 'الرئيسية', to: '/' }, { label: title }]" />
        <LEyebrow class="mt-10">من المدونة</LEyebrow>
        <h1 class="mt-3 text-h2 text-ink">أفكار لإدارة أذكى لمحلّك.</h1>
        <p class="mt-4 max-w-160 text-lead text-ink-soft">{{ description }}</p>

        <div class="mt-14 grid gap-12 md:grid-cols-3 md:gap-11 lg:mt-18">
          <div v-for="(post, i) in BLOG_POSTS" :key="post.slug" class="relative">
            <span v-if="i > 0" class="absolute inset-y-0 -inset-s-5.5 hidden w-px bg-ink/10 md:block" />
            <LBlogCard :post="post" heading-level="h2" />
          </div>
        </div>
      </LContainer>
    </LPagePanel>
    <CtaSection />
  </main>
</template>
