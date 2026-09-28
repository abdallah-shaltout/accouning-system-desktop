<script setup lang="ts">
import { ref, watchEffect } from 'vue';
import { barcodeSvg } from '@/modules/products/helpers/labelBarcode';

/** Code 128 of a document number, drawn by some templates (ticket stub, paper receipt). Decorative-but-real: it scans back to the invoice number. */
const props = withDefaults(defineProps<{ value: string; tone?: 'ink' | 'paper' }>(), { tone: 'ink' });

const svg = ref<string | null>(null);
watchEffect(async () => {
  svg.value = await barcodeSvg(props.value);
});
</script>

<template>
  <!-- eslint-disable-next-line vue/no-v-html -- bwip-js output, generated locally from the invoice number -->
  <div v-if="svg" class="doc-barcode" :class="tone === 'paper' ? 'invert' : ''" role="img" :aria-label="`باركود ${value}`" v-html="svg" />
</template>

<style scoped>
.doc-barcode :deep(svg) {
  display: block;
  width: 100%;
  height: 100%;
}
</style>
