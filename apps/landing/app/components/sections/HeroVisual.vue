<script setup lang="ts">
/**
 * The hero's abstract art (reference: warm reeded glass, a curved glass sheet and a halftone
 * sphere), rebuilt in SVG/CSS so it is crisp, tiny and animatable. Mirrored for RTL: the sphere
 * sits bottom-left, the reeded light top-left. The halftone dots are real sphere geometry
 * (latitude/longitude projected orthographically), generated once at build time.
 */
interface Dot { x: number; y: number; r: number; o: number }

const R = 500
const dots: Dot[] = (() => {
  const out: Dot[] = []
  const tiltX = (28 * Math.PI) / 180
  const tiltY = (-30 * Math.PI) / 180
  const bands = 52
  for (let i = 1; i < bands; i++) {
    const lat = -Math.PI / 2 + (Math.PI * i) / bands
    const count = Math.max(6, Math.round(Math.cos(lat) * 104))
    for (let j = 0; j < count; j++) {
      const lon = (2 * Math.PI * j) / count
      let x = Math.cos(lat) * Math.cos(lon)
      let y = Math.sin(lat)
      let z = Math.cos(lat) * Math.sin(lon)
      ;[y, z] = [y * Math.cos(tiltX) - z * Math.sin(tiltX), y * Math.sin(tiltX) + z * Math.cos(tiltX)]
      ;[x, z] = [x * Math.cos(tiltY) + z * Math.sin(tiltY), -x * Math.sin(tiltY) + z * Math.cos(tiltY)]
      if (z <= 0.04) continue
      const px = R + x * R
      const py = R - y * R
      // Only the quadrant that shows inside the panel matters; dots fade toward the panel centre.
      if (px < 480 || py > 460) continue
      const toCorner = Math.hypot((px - 530) / 1000, (py - 400) / 1000)
      const o = Math.min(1, Math.max(0, 1.05 - toCorner * 1.9))
      if (o < 0.06) continue
      out.push({ x: +px.toFixed(1), y: +py.toFixed(1), r: +(0.9 + z * 2.3).toFixed(2), o: +o.toFixed(2) })
    }
  }
  return out
})()
</script>

<template>
  <div class="hero-art pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
    <div data-hero-layer="reeds" class="hero-reeds absolute inset-y-0 inset-s-[52%] inset-e-[8%]" />
    <div data-hero-layer="beam" class="hero-beam absolute top-0 bottom-[34%] inset-e-[20%] w-[6%]" />

    <svg class="absolute inset-0 size-full" viewBox="0 0 1380 880" preserveAspectRatio="xMidYMax slice">
      <defs>
        <linearGradient id="hero-sheet-fill" x1="1" y1="0.8" x2="0" y2="0.2">
          <stop offset="0%" stop-color="white" stop-opacity=".5" />
          <stop offset="55%" stop-color="white" stop-opacity=".22" />
          <stop offset="100%" stop-color="white" stop-opacity=".04" />
        </linearGradient>
      </defs>
      <path data-hero-layer="sheet" d="M636 900 L636 668 Q636 618 600 584 L-60 -40 L-60 900 Z" fill="url(#hero-sheet-fill)" />
    </svg>

    <div data-hero-layer="sphere" class="absolute inset-e-[-41%] bottom-[-73%] aspect-square w-[78%]">
      <svg viewBox="0 0 1000 1000" class="size-full">
        <defs>
          <radialGradient id="hero-sphere-fill" cx="52%" cy="42%" r="50%">
            <stop offset="0%" stop-color="var(--color-coral-700)" />
            <stop offset="34%" stop-color="var(--color-coral-600)" />
            <stop offset="56%" stop-color="var(--color-coral-500)" />
            <stop offset="78%" stop-color="var(--color-coral-400)" stop-opacity=".55" />
            <stop offset="100%" stop-color="var(--color-coral-300)" stop-opacity="0" />
          </radialGradient>
          <linearGradient id="hero-sphere-fade" x1="0.5" y1="0.45" x2="0.95" y2="0.05">
            <stop offset="0%" stop-color="white" />
            <stop offset="40%" stop-color="white" />
            <stop offset="100%" stop-color="white" stop-opacity="0" />
          </linearGradient>
          <mask id="hero-sphere-mask">
            <rect width="1000" height="1000" fill="url(#hero-sphere-fade)" />
          </mask>
        </defs>
        <g mask="url(#hero-sphere-mask)">
          <circle cx="500" cy="500" r="500" fill="url(#hero-sphere-fill)" />
          <g fill="white">
            <circle v-for="(d, i) in dots" :key="i" :cx="d.x" :cy="d.y" :r="d.r" :opacity="d.o" />
          </g>
        </g>
      </svg>
    </div>

    <!-- Curved glass sheet: vertical edge rising from the floor, bending into a diagonal toward the top corner. -->
    <svg data-hero-layer="edge" class="absolute inset-0 size-full" viewBox="0 0 1380 880" preserveAspectRatio="xMidYMax slice">
      <defs>
        <linearGradient id="hero-sheet-glow" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stop-color="var(--color-coral-300)" stop-opacity="0" />
          <stop offset="60%" stop-color="var(--color-coral-300)" stop-opacity=".8" />
          <stop offset="100%" stop-color="var(--color-coral-400)" stop-opacity=".9" />
        </linearGradient>
        <filter id="hero-sheet-blur" x="-10%" y="-10%" width="120%" height="120%">
          <feGaussianBlur stdDeviation="6" />
        </filter>
      </defs>
      <path d="M646 900 L646 668 Q646 614 608 578 L-50 -46" fill="none" stroke="url(#hero-sheet-glow)" stroke-width="12" filter="url(#hero-sheet-blur)" />
      <path d="M636 900 L636 668 Q636 618 600 584 L-60 -40" fill="none" stroke="white" stroke-opacity=".95" stroke-width="2.5" />
      <path d="M626 900 L626 670 Q626 624 592 592 L-66 -32" fill="none" stroke="white" stroke-opacity=".5" stroke-width="1" />
    </svg>

    <div data-hero-layer="streak" class="hero-streak absolute inset-s-[-6%] bottom-[-2%] h-[9%] w-[42%] rotate-[-7deg]" />
  </div>
</template>

<style scoped>
.hero-art {
  background:
    radial-gradient(40% 45% at 0% 100%, color-mix(in oklab, var(--color-coral-500) 22%, transparent), transparent 70%),
    linear-gradient(180deg, var(--color-hero-50), var(--color-hero-100) 70%, var(--color-hero-200));
}
.hero-reeds {
  background: repeating-linear-gradient(
    90deg,
    color-mix(in oklab, white 60%, transparent) 0 2px,
    color-mix(in oklab, var(--color-hero-200) 70%, transparent) 2px 7px
  );
  mask-image: linear-gradient(180deg, black 10%, transparent 62%);
  opacity: 0.95;
}
.hero-beam {
  background: linear-gradient(
    90deg,
    transparent,
    color-mix(in oklab, var(--color-coral-300) 60%, transparent) 38%,
    color-mix(in oklab, white 85%, transparent) 50%,
    color-mix(in oklab, var(--color-coral-300) 60%, transparent) 62%,
    transparent
  );
  mask-image: linear-gradient(180deg, black 15%, transparent);
  filter: blur(5px);
  opacity: 0.75;
}
.hero-streak {
  background: linear-gradient(90deg, transparent, color-mix(in oklab, var(--color-coral-300) 70%, transparent) 55%, color-mix(in oklab, white 85%, transparent) 75%, transparent);
  filter: blur(3px);
  border-radius: 999px;
  opacity: 0.8;
}
</style>
