<script setup lang="ts">
/** Isometric stack of frosted Equal cards (reference: the stacked "Nero" cards), pure CSS 3D. */
const LAYERS = 4
</script>

<template>
  <div class="stack pointer-events-none relative h-full w-full" aria-hidden="true">
    <div
      v-for="n in LAYERS"
      :key="n"
      data-stack-card
      class="card absolute inset-e-0 top-1/2"
      :style="{ '--i': LAYERS - n }"
    >
      <div class="card-face">
        <span class="card-chip">
          <LLogo tone="white" class="text-[2.6rem] opacity-80" />
        </span>
        <span class="card-word font-display">{{ APP_SHORT_AR }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.stack { perspective: 2400px; }
.card {
  width: min(36rem, 90vw);
  aspect-ratio: 1.6;
  transform-style: preserve-3d;
  transform: translateY(-50%) rotateX(58deg) rotateZ(38deg) translateZ(calc(var(--i) * -3.4rem * (1 + var(--spread, 0) * 0.7)));
  transform-origin: center;
}
.card-face {
  position: absolute;
  inset: 0;
  border-radius: 2.25rem;
  overflow: hidden;
  background:
    radial-gradient(60% 80% at 70% 20%, color-mix(in oklab, white 55%, transparent), transparent 70%),
    linear-gradient(135deg, color-mix(in oklab, white 42%, var(--color-forest-800)), color-mix(in oklab, white 14%, var(--color-forest-900)));
  border: 1px solid color-mix(in oklab, white 35%, transparent);
  box-shadow:
    inset 0 1px 0 color-mix(in oklab, white 50%, transparent),
    0 40px 60px -30px color-mix(in oklab, black 60%, transparent);
  opacity: calc(1 - var(--i) * 0.14);
}
.card-chip {
  position: absolute;
  top: 13%;
  inset-inline-start: 11%;
}
.card-word {
  position: absolute;
  bottom: 12%;
  inset-inline-end: 9%;
  font-size: clamp(3.5rem, 7vw, 6rem);
  line-height: 1;
  font-weight: 600;
  color: color-mix(in oklab, white 92%, transparent);
}
</style>
