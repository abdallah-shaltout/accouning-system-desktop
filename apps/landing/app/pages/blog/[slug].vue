<script setup lang="ts">
import { BLOG_POSTS, findPost } from '~/data/blog'

const route = useRoute()
const post = findPost(String(route.params.slug))
if (!post) throw createError({ statusCode: 404, statusMessage: 'المقال غير موجود', fatal: true })

const related = BLOG_POSTS.filter((p) => p.slug !== post.slug)
const url = `/blog/${post.slug}`

useSeoMeta({
  title: post.title,
  description: post.description,
  ogTitle: post.title,
  ogDescription: post.description,
  ogType: 'article',
  ogImage: '/og.png',
  articlePublishedTime: post.date,
})
useSchemaOrg([
  defineWebPage({ '@type': ['WebPage', 'FAQPage'], name: post.title, description: post.description }),
  defineArticle({
    headline: post.title,
    description: post.description,
    datePublished: post.date,
    dateModified: post.date,
    inLanguage: 'ar',
    image: '/og.png',
  }),
  defineBreadcrumb({
    itemListElement: [
      { name: 'الرئيسية', item: '/' },
      { name: 'المدونة', item: '/blog' },
      { name: post.title, item: url },
    ],
  }),
  ...post.faq.map((f) => defineQuestion({ name: f.q, acceptedAnswer: f.a })),
])

const root = ref<HTMLElement | null>(null)
useLandingMotion(root, ({ gsap }) => {
  gsap.from('[data-article-head] > *', { autoAlpha: 0, y: 24, duration: 1, stagger: 0.08, ease: 'expo.out' })
  gsap.from('[data-article-cover]', { autoAlpha: 0, y: 40, scale: 0.98, duration: 1.2, delay: 0.2, ease: 'expo.out' })
})
</script>

<template>
  <main id="main" ref="root">
    <LPagePanel>
      <article>
        <LContainer>
          <div data-article-head class="mx-auto max-w-prose">
            <LBreadcrumb :items="[{ label: 'الرئيسية', to: '/' }, { label: 'المدونة', to: '/blog' }, { label: post!.title }]" />
            <h1 class="mt-10 text-h2 text-ink text-balance">{{ post!.title }}</h1>
            <p class="mt-5 flex flex-wrap items-center gap-x-3 gap-y-1 text-base text-ink-mute">
              <span>نُشر في <time :datetime="post!.date" class="text-ink-soft">{{ post!.dateLabel }}</time></span>
              <span aria-hidden="true">·</span>
              <span>قراءة <span class="num">{{ post!.readMinutes }}</span> دقائق</span>
              <span aria-hidden="true">·</span>
              <span>فريق {{ APP_NAME_AR }}</span>
            </p>
          </div>

          <div data-article-cover class="mx-auto mt-12 aspect-[16/8] max-w-5xl overflow-hidden rounded-card bg-panel-100">
            <BlogCover :variant="post!.cover" :alt="post!.coverAlt" />
          </div>

          <div class="mx-auto mt-14 max-w-prose">
            <section aria-label="الجواب المختصر" class="rounded-card bg-panel-50 p-7">
              <p class="font-display text-sm text-coral-500">الجواب المختصر</p>
              <p class="mt-3 text-lg leading-loose text-ink">{{ post!.answer }}</p>
            </section>

            <BlogArticleBody :blocks="post!.blocks" />

            <section aria-labelledby="article-faq" class="mt-16">
              <h2 id="article-faq" class="text-3xl text-ink">أسئلة شائعة</h2>
              <dl class="mt-6 divide-y divide-ink/10 border-y border-ink/10">
                <div v-for="f in post!.faq" :key="f.q" class="py-6">
                  <dt class="font-display text-xl text-ink">{{ f.q }}</dt>
                  <dd class="mt-2 text-lg leading-relaxed text-ink-soft">{{ f.a }}</dd>
                </div>
              </dl>
            </section>

            <p class="mt-12 text-lg text-ink-soft">
              جرّب ما قرأته بنفسك: <NuxtLink to="/#features" class="text-coral-500 underline underline-offset-4">شوف مميزات ايكوال</NuxtLink>
              أو <NuxtLink to="/#faq" class="text-coral-500 underline underline-offset-4">اقرأ الأسئلة الشائعة</NuxtLink>.
            </p>
          </div>
        </LContainer>
      </article>

      <LContainer class="mt-24">
        <h2 class="text-3xl text-ink">مقالات أخرى</h2>
        <div class="mt-10 grid gap-12 md:grid-cols-2 md:gap-11 lg:grid-cols-3">
          <LBlogCard v-for="p in related" :key="p.slug" :post="p" />
        </div>
      </LContainer>
    </LPagePanel>
    <CtaSection />
  </main>
</template>
