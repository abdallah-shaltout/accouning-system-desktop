<script setup lang="ts">
import { BLOG_POSTS } from '~/data/blog'

const root = ref<HTMLElement | null>(null)

useLandingMotion(root, ({ gsap, ScrollTrigger }) => {
  const el = root.value!
  gsap.from('[data-reveal="eyebrow"]', { autoAlpha: 0, y: 16, scrollTrigger: revealTrigger(el) })
  gsap.from('[data-line]', { yPercent: 110, duration: 1.1, ease: 'expo.out', scrollTrigger: revealTrigger(el) })
  gsap.set('[data-blog-card]', { autoAlpha: 0, y: 60 })
  ScrollTrigger.batch('[data-blog-card]', {
    start: 'top 88%',
    once: true,
    onEnter: (els) => gsap.to(els, { autoAlpha: 1, y: 0, duration: 1.1, stagger: 0.14, ease: 'expo.out' }),
  })
})
</script>

<template>
  <section id="blog" ref="root" aria-labelledby="blog-title" class="pt-28 pb-20 lg:pt-36 lg:pb-24">
    <LContainer>
      <div class="text-center">
        <LEyebrow>من المدونة</LEyebrow>
        <LSplitHeading id="blog-title" :lines="['أفكار لإدارة أذكى لمحلّك.']" class="mt-3 text-h2 text-ink" />
      </div>

      <div class="mt-12 grid gap-12 md:grid-cols-3 md:gap-11 lg:mt-19">
        <div v-for="(post, i) in BLOG_POSTS" :key="post.slug" data-blog-card class="relative">
          <span v-if="i > 0" class="absolute inset-y-0 -inset-s-5.5 hidden w-px bg-ink/10 md:block" />
          <LBlogCard :post="post" />
        </div>
      </div>
    </LContainer>
  </section>
</template>
