<script setup lang="ts">
/** Product card 3 — weekly sales chart (reference: the "Expenses" card). Decorative. */
const periods = ['أسبوع', 'شهر', '3 شهور', 'سنة']
const days = [
  { d: 'س', h: 22 },
  { d: 'ح', h: 48 },
  { d: 'ن', h: 64 },
  { d: 'ث', h: 58 },
  { d: 'ر', h: 80 },
  { d: 'خ', h: 92, today: true },
  { d: 'ج', h: 40 },
]
</script>

<template>
  <div class="mini-card" aria-hidden="true">
    <p class="text-nano font-medium text-white/90">المبيعات</p>
    <div class="mt-2.5 grid grid-cols-4 gap-1 rounded-lg bg-white/15 p-1 text-micro">
      <span
        v-for="(p, i) in periods"
        :key="p"
        class="grid h-7 place-items-center rounded-md"
        :class="i === 0 ? 'bg-white font-medium text-ink' : 'text-white/85'"
      >{{ p }}</span>
    </div>
    <p class="mt-3 text-nano font-medium text-white/85">إجمالي مبيعات الأسبوع</p>
    <div class="mt-1 flex items-end justify-between">
      <p class="flex items-baseline gap-1.5 font-display">
        <span class="num text-figure font-normal">48,610</span>
        <span class="text-sm text-white/80">ج.م</span>
      </p>
      <span class="num mb-1 rounded-full bg-mint-500 px-2 py-0.5 text-nano text-white">+2.3%</span>
    </div>
    <div class="mt-4 flex h-28 items-end justify-between gap-2 px-1">
      <div v-for="day in days" :key="day.d" class="flex h-full flex-1 flex-col items-center justify-end gap-1.5">
        <span
          data-bar
          class="w-full origin-bottom rounded-full"
          :class="day.today ? 'bg-white' : 'bg-white/30'"
          :style="{ height: `${day.h}%` }"
        />
        <span class="text-nano text-white/85">{{ day.d }}</span>
      </div>
    </div>
  </div>
</template>
