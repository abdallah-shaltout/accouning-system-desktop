<script setup lang="ts">
import { computed } from 'vue';
import { encode } from 'uqr';

/** Crisp vector QR code (one SVG path), black on white so scanners and printers both read it. */
const props = withDefaults(defineProps<{ value: string; size?: string; border?: number }>(), { size: '120px', border: 2 });

const qr = computed(() => encode(props.value, { ecc: 'M', border: props.border }));
const path = computed(() => {
  let d = '';
  qr.value.data.forEach((row, y) =>
    row.forEach((on, x) => {
      if (on) d += `M${x},${y}h1v1h-1z`;
    }),
  );
  return d;
});
</script>

<template>
  <svg
    :viewBox="`0 0 ${qr.size} ${qr.size}`"
    :style="{ width: size, height: size }"
    shape-rendering="crispEdges"
    role="img"
    aria-label="رمز الاستجابة السريعة للفاتورة"
    class="block bg-white"
  >
    <path :d="path" fill="#000" />
  </svg>
</template>
