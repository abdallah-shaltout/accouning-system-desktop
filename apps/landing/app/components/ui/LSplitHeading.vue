<script setup lang="ts">
/**
 * Headline split into explicit lines for GSAP line-mask reveals. Lines are authored in the data,
 * never split per character — splitting Arabic per glyph breaks its joining forms.
 */
withDefaults(defineProps<{ as?: 'h1' | 'h2' | 'h3' | 'p'; lines: string[]; dim?: number[]; dimClass?: string }>(), {
  as: 'h2',
  dim: () => [],
  dimClass: 'opacity-45',
})
</script>

<template>
  <component :is="as" class="text-balance" data-reveal="heading">
    <span v-for="(line, i) in lines" :key="i" class="line-mask">
      <span data-line :class="dim.includes(i) ? dimClass : ''">{{ line }}</span>
    </span>
  </component>
</template>
