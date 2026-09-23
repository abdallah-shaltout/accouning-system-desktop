<script setup lang="ts">
import { ref } from 'vue';
import { Search, X } from '@lucide/vue';

withDefaults(defineProps<{ placeholder?: string; kbd?: string }>(), { placeholder: 'بحث…' });
const model = defineModel<string>({ default: '' });
const input = ref<HTMLInputElement>();
defineExpose({ focus: () => input.value?.focus(), select: () => input.value?.select() });
</script>

<template>
  <div class="relative w-full sm:w-72">
    <Search class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-text-secondary" />
    <input
      ref="input"
      v-model="model"
      type="search"
      :placeholder="placeholder"
      class="control ps-8 pe-8 [&::-webkit-search-cancel-button]:hidden"
      @keydown.esc="model = ''"
    />
    <button
      v-if="model"
      type="button"
      aria-label="مسح"
      class="absolute end-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-text-secondary hover:text-text-primary"
      @click="model = ''; input?.focus()"
    >
      <X class="size-3.5" />
    </button>
    <kbd
      v-else-if="kbd"
      class="num pointer-events-none absolute end-2 top-1/2 -translate-y-1/2 rounded border border-border px-1 text-caption text-text-secondary"
    >{{ kbd }}</kbd>
  </div>
</template>
