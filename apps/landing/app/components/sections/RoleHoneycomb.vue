<script setup lang="ts">
import type { Component } from 'vue'
import { Calculator, Package, Receipt, ScanBarcode, Truck, UserCog } from 'lucide-vue-next'
import { ROLE_LABELS } from '~/data/features'

/**
 * Offset tile grid (rows of 3-4-5-4-3) from the reference's "trusted" cluster. Portrait tiles
 * become the app's real roles; the centre tile holds the Equal mark; the rest stay hatched.
 */
type Tile = { kind: 'empty' } | { kind: 'logo' } | { kind: 'role'; icon: Component; name: string; tone: 'warm' | 'mono' | 'forest' | 'cream'; label?: 'start' | 'end' }

const E: Tile = { kind: 'empty' }
const ROWS: Tile[][] = [
  [E, E, E],
  [E, { kind: 'role', icon: ScanBarcode, name: 'كاشير', tone: 'warm', label: 'start' }, { kind: 'role', icon: Calculator, name: 'محاسب', tone: 'cream' }, E],
  [E, { kind: 'role', icon: Package, name: 'أمين مخزن', tone: 'mono' }, { kind: 'logo' }, { kind: 'role', icon: Receipt, name: 'مبيعات', tone: 'forest' }, E],
  [E, { kind: 'role', icon: Truck, name: 'مشتريات', tone: 'cream' }, { kind: 'role', icon: UserCog, name: 'مدير', tone: 'mono', label: 'end' }, E],
  [E, E, E],
]
</script>

<template>
  <div class="honeycomb mx-auto flex w-max flex-col items-center gap-(--gap)" aria-hidden="true">
    <div v-for="(row, ri) in ROWS" :key="ri" class="flex gap-(--gap)">
      <div v-for="(tile, ti) in row" :key="ti" data-tile class="tile relative">
        <div v-if="tile.kind === 'empty'" class="hatch size-full rounded-[28%] border border-dashed border-ink/15" />
        <div v-else-if="tile.kind === 'logo'" class="grid size-full place-items-center rounded-[28%] border border-panel-200 bg-panel-100 text-[2.6rem]">
          <LLogo tone="ink" />
        </div>
        <div v-else class="role-tile grid size-full place-items-center rounded-[28%] ring-1 ring-ink/10" :class="`tone-${tile.tone}`">
          <component :is="tile.icon" class="size-[38%]" :stroke-width="1.4" />
        </div>

        <div
          v-if="tile.kind === 'role' && tile.label"
          data-tile-label
          class="absolute z-10 flex items-start gap-0.5"
          :class="tile.label === 'start' ? 'top-[52%] inset-e-[90%] flex-row-reverse' : 'top-[70%] inset-s-[88%]'"
        >
          <svg class="-mt-3 size-7 shrink-0 text-ink/85" :class="tile.label === 'end' ? '-scale-x-100' : ''" viewBox="0 0 24 24"><path d="M4 3l16 7-7 2.4L10.6 20z" fill="currentColor" /></svg>
          <span class="rounded-full bg-ink/80 px-5 py-2.5 font-display text-lg whitespace-nowrap text-white backdrop-blur">
            {{ tile.label === 'start' ? ROLE_LABELS.start : ROLE_LABELS.end }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.honeycomb { --tile: clamp(3.4rem, 9.2vw, 8rem); --gap: clamp(0.35rem, 0.85vw, 0.75rem); }
.tile { width: var(--tile); height: var(--tile); }
.tone-warm { background: linear-gradient(160deg, var(--color-coral-300), var(--color-coral-600)); color: white; }
.tone-mono { background: linear-gradient(160deg, var(--color-panel-100), var(--color-panel-300)); color: var(--color-ink); }
.tone-forest { background: linear-gradient(160deg, var(--color-forest-800), var(--color-forest-950)); color: var(--color-lime-400); }
.tone-cream { background: linear-gradient(160deg, var(--color-cream), var(--color-coral-300)); color: var(--color-coral-700); }
</style>
