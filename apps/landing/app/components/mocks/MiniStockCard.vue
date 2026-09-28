<script setup lang="ts">
import { Coffee, Droplet, Package, Wheat } from 'lucide-vue-next'

/** Product card 2 — stock levels (reference: the "Savings" goals card). Decorative. */
const rows = [
  { icon: Wheat, name: 'أرز مصري 5 كجم', qty: '420 عبوة', fill: 72 },
  { icon: Droplet, name: 'زيت عباد الشمس', qty: '186 زجاجة', fill: 46 },
  { icon: Package, name: 'سكر أبيض 1 كجم', qty: '38 كيس', fill: 14 },
  { icon: Coffee, name: 'شاي ناعم', qty: '264 علبة', fill: 60 },
]
</script>

<template>
  <div class="mini-card" aria-hidden="true">
    <p class="text-nano font-medium text-white/90">قيمة المخزون</p>
    <div class="mt-3 flex items-end justify-between">
      <p class="flex items-baseline gap-1.5 font-display">
        <span class="num text-figure font-normal">84,320</span>
        <span class="text-sm text-white/80">ج.م</span>
      </p>
      <span class="num mb-1 rounded-full bg-mint-500 px-2 py-0.5 text-nano text-white">+12%</span>
    </div>
    <ul class="mt-3 space-y-1 rounded-xl bg-white/15 p-2.5">
      <li v-for="row in rows" :key="row.name" class="flex items-center gap-2.5 py-1">
        <span class="grid size-8 shrink-0 place-items-center rounded-full border border-white/45">
          <component :is="row.icon" class="size-3.5" :stroke-width="1.75" />
        </span>
        <span class="min-w-0 flex-1">
          <span class="flex items-center justify-between text-micro">
            <span class="truncate text-white/90">{{ row.name }}</span>
            <span class="num text-white">{{ row.qty.split(' ')[0] }}</span>
          </span>
          <span class="mt-1.5 block h-1.5 overflow-hidden rounded-full bg-white/30">
            <span data-fill class="block h-full rounded-full" :class="row.fill < 20 ? 'bg-cream' : 'bg-white'" :style="{ width: `${row.fill}%` }" />
          </span>
        </span>
      </li>
    </ul>
  </div>
</template>
