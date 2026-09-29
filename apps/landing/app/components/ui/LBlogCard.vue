<script setup lang="ts">
import type { BlogPost } from '~/data/blog'

defineProps<{ post: BlogPost; headingLevel?: 'h2' | 'h3' }>()
</script>

<template>
  <article class="group relative flex flex-col">
    <div class="aspect-4/5 overflow-hidden rounded-mini bg-panel-100">
      <div class="size-full transition-transform duration-700 ease-out-expo group-hover:scale-[1.04]">
        <BlogCover :variant="post.cover" :alt="post.coverAlt" />
      </div>
    </div>
    <component :is="headingLevel ?? 'h3'" class="mt-6 max-w-96 font-display text-h3 leading-snug text-ink transition-colors duration-300 group-hover:text-coral-500">
      <NuxtLink :to="`/blog/${post.slug}`" class="after:absolute after:inset-0">{{ post.title }}</NuxtLink>
    </component>
    <p class="mt-3 line-clamp-2 max-w-96 text-base leading-relaxed text-ink-soft">{{ post.description }}</p>
    <p class="mt-5 flex items-center gap-2 text-eyebrow text-ink-mute">
      <time :datetime="post.date" class="text-ink">{{ post.dateLabel }}</time>
      <span aria-hidden="true">·</span>
      <span><span class="num">{{ post.readMinutes }}</span> دقائق قراءة</span>
    </p>
  </article>
</template>
