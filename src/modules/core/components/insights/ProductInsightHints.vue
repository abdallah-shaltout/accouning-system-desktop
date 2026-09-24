<script setup lang="ts">
import { useProductInlineHints } from '@/modules/core/controllers/useInsights';
import InsightChip from './InsightChip.vue';

/** Inline hints for a product detail page (docs/v2/11 D1), e.g. "هذا المنتج منخفض المخزون". */
const props = defineProps<{ productId?: string }>();
const insights = useProductInlineHints(() => props.productId);
</script>

<template>
  <div v-if="insights.length" class="flex flex-wrap gap-1.5">
    <InsightChip v-for="i in insights" :key="i.id" :insight="i" />
  </div>
</template>
