<script setup lang="ts">
import { ChevronDown, KeyRound, Plus } from 'lucide-vue-next'

/** Manager approval + customer credit limit (reference: "Transfer money" + "Spending limit"). Decorative. */
</script>

<template>
  <div class="flex w-full max-w-84 flex-col gap-4" aria-hidden="true">
    <div class="rounded-card border border-panel-200 bg-white p-3.5 shadow-soft ring-4 ring-white/60">
      <p class="text-micro font-medium text-ink">طلب موافقة المدير</p>
      <div class="mt-3 flex items-center gap-2.5 rounded-xl border border-panel-100 bg-panel-50 p-2.5">
        <span class="grid size-8 shrink-0 place-items-center rounded-full bg-cream font-display text-xs text-coral-600 ring-2 ring-coral-300/60">سا</span>
        <span class="flex-1 leading-tight">
          <span class="block text-micro text-ink">سارة <span class="text-ink-soft">(كاشير) تطلب خصم</span></span>
          <span class="block text-nano text-ink-mute">منذ دقيقتين</span>
        </span>
        <span class="grid size-7 place-items-center rounded-full bg-coral-500 text-white"><Plus class="size-4" /></span>
      </div>
      <p class="mt-7 flex items-baseline justify-center gap-2 font-display text-ink">
        <span class="text-lg">ج.م</span>
        <span class="num text-6xl leading-none">185</span>
      </p>
      <p class="mt-5 text-center text-nano text-ink-soft">خصم <span class="num">15%</span> على فاتورة <span class="num">#1052</span></p>
      <div class="mx-auto mt-3 flex w-max items-center gap-2 rounded-full border border-panel-200 px-3 py-1.5 text-nano text-ink">
        <KeyRound class="size-3.5 text-coral-500" />
        رمز المدير <span class="num tracking-widest">••••</span>
        <ChevronDown class="size-3.5 text-ink-mute" />
      </div>
      <span class="mt-4 grid h-9 place-items-center rounded-full bg-linear-to-b from-coral-400 to-coral-500 text-micro font-medium text-white">اعتماد الخصم</span>
    </div>

    <div class="rounded-card border border-panel-200 bg-white p-3.5 shadow-soft ring-4 ring-white/60">
      <p class="text-micro font-medium text-ink">حد الائتمان · مؤسسة الفجر</p>
      <p class="mt-3 text-nano text-ink-soft">المستخدم من الحد</p>
      <p class="mt-1 flex items-baseline gap-1.5 font-display text-ink">
        <span class="text-sm">ج.م</span>
        <span class="num text-[2rem] leading-none">10,220</span>
      </p>
      <span class="mt-3 block h-4 overflow-hidden rounded-full bg-panel-100">
        <span data-credit-fill class="block h-full w-[47%] rounded-full bg-linear-to-l from-coral-400 to-coral-500" />
      </span>
    </div>
  </div>
</template>
