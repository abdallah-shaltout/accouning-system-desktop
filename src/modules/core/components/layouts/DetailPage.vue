<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 (F1 layouts table — "DetailPage"): `DetailHeader` · optional `StatCards` · tabs
 * (details / lines / payments / history / attachments — whichever the page declares) · aside.
 *
 * Not wired into any existing detail page yet (F-4 migrates invoice/purchase/party/product/journal
 * entry/shift detail pages — out of scope for F-0); this is the shell those pages will assemble once
 * that batch starts.
 */
import { ref } from 'vue';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/modules/core/components/shadcn/tabs';
import DetailHeader, { type MetaChip } from '@/modules/core/components/blocks/DetailHeader.vue';
import StatCards, { type StatCard } from '@/modules/core/components/blocks/StatCards.vue';
import type { Tone } from '@/modules/core/helpers/labels';
import type { RouteLocationRaw } from 'vue-router';

export interface DetailTab {
  key: string;
  label: string;
}

const props = withDefaults(
  defineProps<{
    title: string;
    number?: string;
    status?: { label: string; tone: Tone };
    chips?: MetaChip[];
    back?: RouteLocationRaw;
    stats?: StatCard[];
    /** Omit for a single-section page (no tab bar rendered). */
    tabs?: DetailTab[];
    defaultTab?: string;
  }>(),
  {},
);

const activeTab = ref(props.defaultTab ?? props.tabs?.[0]?.key ?? '');
</script>

<template>
  <div>
    <DetailHeader :title="title" :number="number" :status="status" :chips="chips" :back="back">
      <template #actions><slot name="actions" /></template>
    </DetailHeader>

    <StatCards v-if="stats?.length" :cards="stats" class="mb-5" />

    <div class="grid grid-cols-1 gap-6" :class="$slots.aside && 'lg:grid-cols-[1fr_300px]'">
      <div class="min-w-0">
        <Tabs v-if="tabs?.length" v-model="activeTab">
          <TabsList>
            <TabsTrigger v-for="t in tabs" :key="t.key" :value="t.key">{{ t.label }}</TabsTrigger>
          </TabsList>
          <TabsContent v-for="t in tabs" :key="t.key" :value="t.key">
            <slot :name="`tab-${t.key}`" />
          </TabsContent>
        </Tabs>
        <slot v-else />
      </div>
      <aside v-if="$slots.aside" class="space-y-4">
        <slot name="aside" />
      </aside>
    </div>
  </div>
</template>
