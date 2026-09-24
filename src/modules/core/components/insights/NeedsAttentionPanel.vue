<script setup lang="ts">
import { ref } from 'vue';
import { CircleCheck } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import { useInsights } from '@/modules/core/controllers/useInsights';
import InsightCard from './InsightCard.vue';

/** Home "يحتاج انتباهك" (docs/v2/11 Part B.2): top 5 insights + a drawer for the rest. */
withDefaults(defineProps<{ limit?: number }>(), { limit: 5 });

const { insights: allInsights, dismiss, snooze } = useInsights();
const drawerOpen = ref(false);
</script>

<template>
  <AppCard title="يحتاج انتباهك" padding="sm">
    <template v-if="allInsights.length > limit" #actions>
      <AppButton size="sm" variant="ghost" @click="drawerOpen = true">عرض الكل ({{ allInsights.length }})</AppButton>
    </template>

    <div v-if="!allInsights.length" class="flex flex-col items-center justify-center gap-2 py-8 text-center">
      <CircleCheck class="size-8 text-success" :stroke-width="1.5" />
      <p class="text-body font-medium">كل شيء على ما يرام ✓</p>
    </div>
    <div v-else class="grid gap-2.5 sm:grid-cols-2 xl:grid-cols-3">
      <InsightCard
        v-for="i in allInsights.slice(0, limit)"
        :key="i.id"
        :insight="i"
        @dismiss="dismiss(i)"
        @snooze="snooze(i)"
      />
    </div>

    <AppModal v-model:open="drawerOpen" title="كل التوصيات" size="lg">
      <div class="space-y-2.5">
        <InsightCard v-for="i in allInsights" :key="i.id" :insight="i" @dismiss="dismiss(i)" @snooze="snooze(i)" />
      </div>
      <template #footer>
        <AppButton @click="drawerOpen = false">إغلاق</AppButton>
      </template>
    </AppModal>
  </AppCard>
</template>
