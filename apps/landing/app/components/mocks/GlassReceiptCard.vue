<script setup lang="ts">
/** Dark-glass receipt overlay pinned on the feature scene (reference: the "+4.127" transaction card). */
</script>

<template>
  <div
    class="w-full max-w-83 rounded-mini border border-white/45 bg-white/12 p-3.5 text-white backdrop-blur-xl shadow-glass"
    aria-hidden="true"
  >
    <p class="text-nano font-medium text-white/90">إيصال بيع</p>
    <div class="mt-2.5 flex items-start justify-between gap-3">
      <div>
        <p class="flex items-baseline gap-1.5 font-display">
          <span class="num text-figure font-normal">+4,127</span>
          <span class="text-sm text-white/80">ج.م</span>
        </p>
        <p class="mt-1.5 text-micro text-white/90">متجر النور · كاشير <span class="num">2</span></p>
        <p class="mt-0.5 text-nano text-white/70">منذ دقيقتين</p>
      </div>
      <span class="grid size-10 shrink-0 place-items-center rounded-full bg-cream font-display text-sm font-medium text-coral-600 ring-2 ring-lime-400/80">من</span>
    </div>
    <div class="mt-3 grid grid-cols-2 gap-2 text-micro">
      <span class="grid h-8 place-items-center rounded-full bg-linear-to-b from-coral-400 to-coral-500 font-medium">مشاركة</span>
      <span class="grid h-8 place-items-center rounded-full bg-white font-medium text-ink">إلغاء</span>
    </div>
    <div class="mini-row mt-2 bg-white/10">
      <span>طباعة بدون شعار</span>
      <MiniSwitch tone="coral" />
    </div>
    <div class="mini-row mt-1.5 h-auto! bg-white/10 py-2">
      <span class="flex flex-col gap-0.5">
        <span>الحالة</span>
        <span class="text-nano text-white/70">يُرحّل تلقائيًا عند إغلاق الوردية</span>
      </span>
      <span class="flex items-center gap-1 text-coral-300">
        <svg class="size-3 animate-spin-slow" viewBox="0 0 12 12"><path d="M6 1a5 5 0 1 0 5 5" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /></svg>
        قيد الترحيل
      </span>
    </div>
    <div class="mini-row mt-1.5 bg-white/10">
      <span>الإبلاغ عن مشكلة</span>
    </div>
  </div>
</template>
