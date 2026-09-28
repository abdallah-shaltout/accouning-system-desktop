<script setup lang="ts">
/**
 * Source of public/og.png (1200×630). Not linked, not in the sitemap, noindex. Regenerate with:
 *   python scripts/og.py   (dev server running on :3100)
 */
definePageMeta({ bare: true })
useSeoMeta({ robots: 'noindex, nofollow', title: 'OG' })
</script>

<template>
  <div class="og relative h-[630px] w-[1200px] overflow-hidden bg-coral-500 p-2.5">
    <div class="relative flex size-full overflow-hidden rounded-panel bg-hero-100">
      <HeroVisual />
      <div class="relative z-10 flex h-full w-[64%] flex-col justify-between p-14">
        <LLogo wordmark class="text-3xl" />
        <div>
          <p class="text-2xl font-medium text-coral-500">برنامج المحاسبة لمحلّك</p>
          <p class="mt-4 font-display text-[3.75rem] leading-[1.2] font-medium whitespace-nowrap text-ink">حساباتك مضبوطة.<br>وبياناتك في محلّك.</p>
          <p class="mt-5 text-2xl text-ink-soft">فواتير · كاشير · مخزون · تقارير — بدون إنترنت</p>
        </div>
      </div>
      <div class="absolute inset-e-10 top-1/2 z-10 w-[21rem] -translate-y-1/2 rotate-[-4deg]">
        <MiniReportCard class="max-w-none! bg-coral-500/90 shadow-float backdrop-blur" />
      </div>
    </div>
  </div>
</template>
