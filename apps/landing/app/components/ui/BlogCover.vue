<script setup lang="ts">
import type { BlogPost } from '~/data/blog'

/**
 * Editorial covers drawn in SVG (reference palettes: cyan still-life, pastel chart, orange
 * architecture). Vector keeps them crisp and weightless; `alt` comes from the post data.
 */
defineProps<{ variant: BlogPost['cover']; alt: string }>()

const bars = [0.35, 0.5, 0.42, 0.62, 0.55, 0.78, 0.7, 0.9]
</script>

<template>
  <svg viewBox="0 0 400 580" class="size-full" role="img" :aria-label="alt" preserveAspectRatio="xMidYMid slice">
    <template v-if="variant === 'receipt'">
      <rect width="400" height="580" fill="#9FDCE8" />
      <path d="M0 470 C80 440 120 520 200 500 S330 440 400 470 V580 H0Z" fill="#1E8FA3" opacity=".55" />
      <g transform="rotate(-12 200 290)">
        <path d="M120 120 h160 v300 l-14 -10 -14 10 -14 -10 -14 10 -14 -10 -14 10 -14 -10 -14 10 -14 -10 -14 10 -14 -10 -12 10z" fill="#FFFFFF" />
        <path d="M120 120 h160 v300 l-14 -10 -14 10 -14 -10 -14 10 -14 -10 -14 10 -14 -10 -14 10 -14 -10 -14 10 -14 -10 -12 10z" fill="none" stroke="#0E6F80" stroke-opacity=".15" />
        <rect x="150" y="150" width="100" height="10" rx="5" fill="#0B3B44" opacity=".75" />
        <rect x="170" y="170" width="60" height="6" rx="3" fill="#0B3B44" opacity=".35" />
        <g fill="#0B3B44" opacity=".4">
          <rect x="145" y="205" width="80" height="6" rx="3" /><rect x="235" y="205" width="22" height="6" rx="3" />
          <rect x="145" y="225" width="64" height="6" rx="3" /><rect x="235" y="225" width="22" height="6" rx="3" />
          <rect x="145" y="245" width="90" height="6" rx="3" /><rect x="235" y="245" width="22" height="6" rx="3" />
          <rect x="145" y="265" width="56" height="6" rx="3" /><rect x="235" y="265" width="22" height="6" rx="3" />
        </g>
        <rect x="145" y="292" width="112" height="1.5" fill="#0B3B44" opacity=".3" />
        <rect x="145" y="305" width="50" height="9" rx="4" fill="#F3553B" />
        <rect x="215" y="305" width="42" height="9" rx="4" fill="#0B3B44" opacity=".75" />
        <g fill="#0B3B44" opacity=".8">
          <rect x="150" y="345" width="3" height="40" /><rect x="157" y="345" width="1.5" height="40" /><rect x="162" y="345" width="4" height="40" />
          <rect x="170" y="345" width="2" height="40" /><rect x="176" y="345" width="3" height="40" /><rect x="183" y="345" width="1.5" height="40" />
          <rect x="188" y="345" width="4" height="40" /><rect x="196" y="345" width="2" height="40" /><rect x="202" y="345" width="3" height="40" />
          <rect x="209" y="345" width="1.5" height="40" /><rect x="214" y="345" width="4" height="40" /><rect x="222" y="345" width="2" height="40" />
          <rect x="228" y="345" width="3" height="40" /><rect x="235" y="345" width="1.5" height="40" /><rect x="240" y="345" width="4" height="40" />
        </g>
      </g>
      <ellipse cx="210" cy="500" rx="120" ry="14" fill="#0B3B44" opacity=".18" />
    </template>

    <template v-else-if="variant === 'chart'">
      <defs>
        <radialGradient id="bc-a" cx="20%" cy="15%" r="60%"><stop offset="0" stop-color="#F7B38F" /><stop offset="1" stop-color="#F7B38F" stop-opacity="0" /></radialGradient>
        <radialGradient id="bc-b" cx="80%" cy="85%" r="55%"><stop offset="0" stop-color="#B8A6F2" /><stop offset="1" stop-color="#B8A6F2" stop-opacity="0" /></radialGradient>
        <radialGradient id="bc-c" cx="75%" cy="45%" r="50%"><stop offset="0" stop-color="#BDEFD6" /><stop offset="1" stop-color="#BDEFD6" stop-opacity="0" /></radialGradient>
        <filter id="bc-blur"><feGaussianBlur stdDeviation="3" /></filter>
      </defs>
      <rect width="400" height="580" fill="#F4E6D8" />
      <rect width="400" height="580" fill="url(#bc-a)" />
      <rect width="400" height="580" fill="url(#bc-c)" />
      <rect width="400" height="580" fill="url(#bc-b)" />
      <g opacity=".35" filter="url(#bc-blur)">
        <rect v-for="(b, i) in bars" :key="i" :x="30 + i * 44" :y="560 - b * 300" width="26" :height="b * 300" rx="6" fill="#FFFFFF" />
      </g>
      <path
        d="M-10 470 L40 420 L70 440 L110 360 L150 330 L180 350 L215 280 L245 300 L280 220 L310 240 L345 150 L375 170 L410 110"
        fill="none" stroke="#0F8A5F" stroke-width="6" stroke-linejoin="round" stroke-linecap="round"
      />
      <path
        d="M-10 470 L40 420 L70 440 L110 360 L150 330 L180 350 L215 280 L245 300 L280 220 L310 240 L345 150 L375 170 L410 110 V580 H-10Z"
        fill="#FFFFFF" opacity=".22"
      />
      <circle cx="345" cy="150" r="9" fill="#FFFFFF" stroke="#0F8A5F" stroke-width="4" />
    </template>

    <template v-else>
      <rect width="400" height="580" fill="#F6A07A" />
      <g>
        <path d="M60 170 L230 110 L400 170 V580 H60Z" fill="#F7B996" />
        <path d="M230 110 L400 170 V580 H230Z" fill="#E8612F" />
        <path d="M60 170 L230 110 V580 H60Z" fill="#F9C9A8" />
        <g fill="#E8612F" opacity=".55">
          <rect v-for="r in 8" :key="`l${r}`" x="88" :y="190 + r * 44" width="112" height="10" rx="2" />
        </g>
        <g fill="#FCE4D2">
          <template v-for="r in 8" :key="`w${r}`">
            <rect x="255" :y="195 + r * 44" width="22" height="26" rx="2" opacity=".85" />
            <rect x="295" :y="198 + r * 44" width="22" height="26" rx="2" opacity=".7" />
            <rect x="335" :y="201 + r * 44" width="22" height="26" rx="2" opacity=".55" />
          </template>
        </g>
        <path d="M230 110 V580" stroke="#C9481C" stroke-width="2" opacity=".6" />
      </g>
      <rect width="400" height="580" fill="url(#bc-shade)" />
      <defs>
        <linearGradient id="bc-shade" x1="0" y1="0" x2="0" y2="1"><stop offset=".6" stop-color="#C43A12" stop-opacity="0" /><stop offset="1" stop-color="#C43A12" stop-opacity=".35" /></linearGradient>
      </defs>
    </template>
  </svg>
</template>
