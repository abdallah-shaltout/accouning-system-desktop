<script setup lang="ts">
import { Bell } from 'lucide-vue-next'

/** Dark home dashboard in a window (reference: the dark phone with the lime chart). Decorative. */
const months = [
  { m: 'أبريل', a: 38, b: 20 },
  { m: 'مايو', a: 56, b: 30 },
  { m: 'يونيو', a: 44, b: 18 },
  { m: 'يوليو', a: 50, b: 26 },
  { m: 'أغسطس', a: 64, b: 34 },
  { m: 'سبتمبر', a: 86, b: 40 },
]
</script>

<template>
  <LWindowMock theme="dark" title="الرئيسية">
    <div class="dash-glow relative px-4 pt-4 pb-5">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <span class="grid size-8 place-items-center rounded-full bg-cream font-display text-xs text-coral-600">مع</span>
          <span class="leading-tight">
            <span class="block text-nano text-white/60">صباح الخير،</span>
            <span class="block text-micro text-white">محمود عبد الله</span>
          </span>
        </div>
        <span class="relative text-white/70">
          <Bell class="size-4" :stroke-width="1.75" />
          <span class="absolute -top-0.5 -inset-e-0.5 size-1.5 rounded-full bg-coral-500" />
        </span>
      </div>

      <div class="mt-4 rounded-xl border border-white/10 bg-white/[0.03] p-3.5">
        <div class="flex items-center justify-between text-nano">
          <span class="text-white/70">صافي ربح الشهر</span>
          <span class="num text-lime-400">YTD +348,120</span>
        </div>
        <p class="mt-2 flex items-baseline gap-1 font-display">
          <span class="num text-[2rem] leading-none">50,320</span>
          <span class="num text-xs text-white/50">.00</span>
          <span class="ms-1 text-xs text-white/60">ج.م</span>
        </p>
        <p class="mt-2 flex items-center gap-2 text-nano">
          <span class="num rounded bg-lime-400/15 px-1.5 py-0.5 text-lime-400">+15.8%</span>
          <span class="text-white/60"><span class="num">+6,840</span> هذا الشهر</span>
        </p>
        <div class="mt-3 grid grid-cols-2 gap-3 border-t border-white/10 pt-3">
          <div>
            <p class="text-nano text-white/50">النقدية بالخزينة</p>
            <p class="mt-1 font-display text-base"><span class="num">47,200<span class="text-xs text-white/50">.00</span></span></p>
            <p class="text-nano text-white/40">جاهزة للإيداع</p>
          </div>
          <div>
            <p class="text-nano text-white/50">مستحقات العملاء</p>
            <p class="mt-1 font-display text-base"><span class="num">8,200<span class="text-xs text-white/50">.00</span></span></p>
            <p class="text-nano text-white/40"><span class="num">9</span> فواتير · <span class="num">30</span> يوم</p>
          </div>
        </div>
      </div>
    </div>

    <div class="bg-white px-4 pt-3.5 pb-4 text-ink">
      <div class="flex items-center justify-between text-micro">
        <span>اتجاه المبيعات</span>
        <span class="text-nano text-ink-mute">آخر <span class="num">6</span> شهور</span>
      </div>
      <div class="mt-2.5 flex h-32 items-end gap-2.5 border-b border-panel-200 pb-px">
        <div v-for="x in months" :key="x.m" class="flex h-full flex-1 flex-col justify-end">
          <span data-dash-bar class="block origin-bottom rounded-t-sm bg-lime-200" :style="{ height: `${x.a - x.b}%` }" />
          <span data-dash-bar class="block origin-bottom bg-lime-500" :style="{ height: `${x.b}%` }" />
        </div>
      </div>
      <div class="mt-1.5 flex gap-2.5 text-center text-nano text-ink-mute">
        <span v-for="x in months" :key="x.m" class="flex-1 truncate">{{ x.m }}</span>
      </div>
      <p class="mt-2.5 flex items-center gap-3 text-nano text-ink-soft">
        <span class="flex items-center gap-1"><span class="size-2 rounded-sm bg-lime-500" />محصّل</span>
        <span class="flex items-center gap-1"><span class="size-2 rounded-sm bg-lime-200" />آجل</span>
      </p>
    </div>
  </LWindowMock>
</template>

<style scoped>
.dash-glow {
  background:
    radial-gradient(55% 60% at 0% 70%, color-mix(in oklab, var(--color-lime-500) 38%, transparent), transparent 70%),
    var(--color-forest-950);
}
</style>
