<script setup lang="ts">
import { computed, ref } from 'vue';
import { formatDate, formatMoney, formatNumber } from '../../helpers/format';

/**
 * Single-series column chart (inline SVG, no library). One hue — today's column in the full
 * accent, past days a lighter step of the same hue. Hover any column band for the exact value.
 * RTL: time runs right → left (oldest on the right), matching Arabic reading order.
 * v2 phase 10 (docs/v2/11 Part B.4): optional `previousTotal` per point draws a light comparison
 * line for the previous period, so the home's one chart shows both series at once.
 */
const props = defineProps<{ data: { date: string; total: number; previousTotal?: number }[] }>();

const hasComparison = computed(() => props.data.some((d) => d.previousTotal !== undefined));

const W = 640;
const H = 200;
const PAD = { top: 12, right: 52, bottom: 26, left: 8 }; // y-axis on the right (the RTL start edge)
const plotW = W - PAD.left - PAD.right;
const plotH = H - PAD.top - PAD.bottom;

const hover = ref<number | null>(null);

/** Clean y ticks: 0, step, 2×step… with a 1/2/5 × 10ⁿ step. */
const ticks = computed(() => {
  const max = Math.max(1, ...props.data.map((d) => Math.max(d.total, d.previousTotal ?? 0)));
  const rough = max / 4;
  const pow = 10 ** Math.floor(Math.log10(rough));
  const step = [1, 2, 5, 10].map((m) => m * pow).find((s) => s >= rough) ?? rough;
  const top = Math.ceil(max / step) * step;
  const list: number[] = [];
  for (let v = 0; v <= top + step / 2; v += step) list.push(v);
  return { list, top };
});

const band = computed(() => plotW / Math.max(1, props.data.length));
const barW = computed(() => Math.min(24, band.value - 6));

const bars = computed(() =>
  props.data.map((d, i) => {
    const h = (Math.max(0, d.total) / ticks.value.top) * plotH;
    // RTL: index 0 (oldest) sits at the right edge.
    const bandX = PAD.left + plotW - (i + 1) * band.value;
    const x = bandX + (band.value - barW.value) / 2;
    const y = PAD.top + plotH - h;
    return { ...d, i, bandX, x, y, h, isToday: i === props.data.length - 1 };
  }),
);

/** Column path: 4px rounded data-end, square at the baseline. */
function columnPath(x: number, y: number, w: number, h: number): string {
  if (h <= 0) return '';
  const r = Math.min(4, h, w / 2);
  const base = y + h;
  return `M${x},${base}V${y + r}Q${x},${y} ${x + r},${y}H${x + w - r}Q${x + w},${y} ${x + w},${y + r}V${base}Z`;
}

function yOf(v: number) {
  return PAD.top + plotH - (v / ticks.value.top) * plotH;
}

/** Previous-period light comparison line, aligned to the same column centers as the bars. */
const comparisonPath = computed(() => {
  if (!hasComparison.value) return '';
  return bars.value
    .map((b, i) => `${i === 0 ? 'M' : 'L'}${round1(b.bandX + band.value / 2)},${round1(yOf(b.previousTotal ?? 0))}`)
    .join(' ');
});

function round1(n: number) {
  return Math.round(n * 10) / 10;
}

function compact(v: number) {
  return v >= 1000 ? `${formatNumber(v / 1000, 1)}K` : formatNumber(v, 0);
}

const dayLabel = (key: string) => {
  const [, m, d] = key.split('-');
  return `${formatNumber(Number(d))}/${formatNumber(Number(m))}`;
};

const tooltip = computed(() => (hover.value === null ? null : bars.value[hover.value]));
</script>

<template>
  <div class="relative">
    <svg :viewBox="`0 0 ${W} ${H}`" class="block h-auto w-full" role="img" aria-label="صافي المبيعات اليومية لآخر 14 يوماً" @mouseleave="hover = null">
      <!-- Gridlines + y ticks (hairline, solid, recessive) -->
      <g>
        <template v-for="t in ticks.list" :key="t">
          <line :x1="PAD.left" :x2="W - PAD.right" :y1="yOf(t)" :y2="yOf(t)" stroke="var(--color-border)" stroke-width="1" />
          <text :x="W - PAD.right + 8" :y="yOf(t) + 4" text-anchor="start" font-size="11" fill="var(--color-text-secondary)" direction="ltr">
            {{ compact(t) }}
          </text>
        </template>
      </g>

      <g v-for="b in bars" :key="b.date">
        <!-- Hover band (hit target larger than the mark) -->
        <rect
          :x="b.bandX"
          :y="PAD.top"
          :width="band"
          :height="plotH"
          :fill="hover === b.i ? 'var(--color-surface-hover)' : 'transparent'"
          @mouseenter="hover = b.i"
        />
        <path
          :d="columnPath(b.x, b.y, barW, b.h)"
          fill="var(--color-primary)"
          :fill-opacity="b.isToday || hover === b.i ? 1 : 0.45"
          pointer-events="none"
        />
        <circle v-if="hover === b.i && hasComparison" :cx="b.bandX + band / 2" :cy="yOf(b.previousTotal ?? 0)" r="2.5" fill="var(--color-text-secondary)" pointer-events="none" />
        <text
          v-if="b.i % 2 === (data.length - 1) % 2"
          :x="b.bandX + band / 2"
          :y="H - 8"
          text-anchor="middle"
          font-size="10.5"
          :fill="b.isToday ? 'var(--color-text-primary)' : 'var(--color-text-secondary)'"
          direction="ltr"
        >
          {{ b.isToday ? 'اليوم' : dayLabel(b.date) }}
        </text>
      </g>

      <!-- Previous-period comparison line (docs/v2/11 Part B.4: "the previous period as a light line") -->
      <path v-if="hasComparison" :d="comparisonPath" fill="none" stroke="var(--color-text-secondary)" stroke-width="1.5" stroke-dasharray="3 3" opacity="0.6" pointer-events="none" />
    </svg>

    <div
      v-if="tooltip"
      class="pointer-events-none absolute top-0 z-10 -translate-x-1/2 rounded-md border border-border bg-background px-2.5 py-1.5 text-xs shadow-lg"
      :style="{ left: `${((tooltip.bandX + band / 2) / W) * 100}%` }"
    >
      <p class="text-text-secondary">{{ formatDate(tooltip.date) }}</p>
      <p class="num font-medium text-text-primary">{{ formatMoney(tooltip.total) }}</p>
      <p v-if="tooltip.previousTotal !== undefined" class="num text-text-secondary">الفترة السابقة: {{ formatMoney(tooltip.previousTotal) }}</p>
    </div>

    <!-- Table view for screen readers -->
    <table class="sr-only">
      <caption>صافي المبيعات اليومية</caption>
      <tr v-for="d in data" :key="d.date">
        <th>{{ d.date }}</th>
        <td>{{ formatMoney(d.total) }}</td>
        <td v-if="d.previousTotal !== undefined">{{ formatMoney(d.previousTotal) }}</td>
      </tr>
    </table>
  </div>
</template>
