<script setup lang="ts">
import { useEntityInsights } from '@/modules/core/controllers/useInsights';
import InsightChip from './InsightChip.vue';

/**
 * Drop-in inline hints for an entity page (product/party/report — docs/v2/11 D1). Pass the rule
 * keys relevant to that entity type and its id; renders nothing when no rule currently fires.
 */
const props = defineProps<{ ruleKeys: string[]; entityId?: string }>();
const insights = useEntityInsights(props.ruleKeys, () => props.entityId);
</script>

<template>
  <div v-if="insights.length" class="flex flex-wrap gap-1.5">
    <InsightChip v-for="i in insights" :key="i.id" :insight="i" />
  </div>
</template>
