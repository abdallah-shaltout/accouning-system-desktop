<script setup lang="ts">
/** Product card 1 — a sale invoice (reference: the "Transaction" card). Decorative. */
</script>

<template>
  <div class="mini-card" aria-hidden="true">
    <p class="text-nano font-medium text-white/90">فاتورة بيع</p>
    <div class="mt-3 flex items-start justify-between gap-3">
      <div>
        <p class="num font-display text-figure font-normal">+1,250</p>
        <p class="mt-2 text-micro text-white/90">أحمد للتجارة · فاتورة <span class="num">#1048</span></p>
        <p class="mt-0.5 text-nano text-white/70">منذ دقيقتين</p>
      </div>
      <span class="grid size-10 shrink-0 place-items-center rounded-full bg-cream font-display text-sm font-medium text-coral-600 ring-2 ring-white/40">أح</span>
    </div>
    <div class="mt-3 grid grid-cols-2 gap-2 text-micro">
      <span class="grid h-8 place-items-center rounded-full bg-white font-medium text-ink">مشاركة</span>
      <span class="grid h-8 place-items-center rounded-full border border-white/40 text-white">إلغاء</span>
    </div>
    <div class="mini-row mt-2">
      <span>طباعة إيصال</span>
      <MiniSwitch />
    </div>
    <div class="mini-row mt-1.5 h-auto! py-2">
      <span class="flex flex-col gap-0.5">
        <span>الحالة</span>
        <span class="text-nano text-white/70">تم التحصيل نقدًا · وردية الصباح</span>
      </span>
      <span class="flex items-center gap-1 text-white">
        <svg class="size-3" viewBox="0 0 12 12"><circle cx="6" cy="6" r="5" fill="none" stroke="currentColor" stroke-width="1.4" /><path d="M3.6 6.2l1.6 1.5 3.2-3.3" fill="none" stroke="currentColor" stroke-width="1.4" /></svg>
        مدفوعة
      </span>
    </div>
    <div class="mini-row mt-1.5">
      <span>إرسال عبر واتساب</span>
    </div>
  </div>
</template>
