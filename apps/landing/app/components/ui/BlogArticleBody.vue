<script setup lang="ts">
import type { BlogBlock } from '~/data/blog'

defineProps<{ blocks: BlogBlock[] }>()
</script>

<template>
  <div class="article-body">
    <template v-for="(b, i) in blocks" :key="i">
      <h2 v-if="b.type === 'h2'" class="mt-14 text-3xl leading-snug text-ink">{{ b.text }}</h2>
      <p v-else-if="b.type === 'p'" class="mt-5 text-lg leading-loose text-ink-soft">{{ b.text }}</p>
      <ol v-else-if="b.type === 'ol'" class="mt-6 space-y-3 ps-6 text-lg leading-relaxed text-ink-soft marker:font-display marker:text-coral-500 list-decimal">
        <li v-for="item in b.items" :key="item" class="ps-2">{{ item }}</li>
      </ol>
      <ul v-else-if="b.type === 'ul'" class="mt-6 space-y-3 ps-6 text-lg leading-relaxed text-ink-soft marker:text-coral-500 list-disc">
        <li v-for="item in b.items" :key="item" class="ps-2">{{ item }}</li>
      </ul>
      <div v-else-if="b.type === 'table'" class="mt-8 overflow-x-auto rounded-mini border border-ink/10">
        <table class="w-full min-w-md text-start text-base">
          <thead class="bg-panel-50">
            <tr>
              <th v-for="h in b.head" :key="h" scope="col" class="px-5 py-3.5 text-start font-display font-medium text-ink">{{ h }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, ri) in b.rows" :key="ri" class="border-t border-ink/10">
              <td v-for="(cell, ci) in row" :key="ci" class="px-5 py-3.5" :class="ci === 0 ? 'text-ink' : 'num text-ink-soft'">{{ cell }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-else-if="b.type === 'note'" class="mt-8 rounded-mini border-s-4 border-coral-500 bg-cream/60 px-6 py-5 text-lg leading-relaxed text-ink">{{ b.text }}</p>
    </template>
  </div>
</template>
