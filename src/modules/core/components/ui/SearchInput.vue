<script setup lang="ts">
/** Rebuilt on shadcn's Input (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — same props/emits as
 * before. (InputGroup wasn't used here: it doesn't support the trailing clear-button/kbd-hint
 * swap this needs, and the manual icon-inset layout below is simpler for that.) */
import { ref } from 'vue';
import { Search, X } from '@lucide/vue';
import { Input } from '@/modules/core/components/shadcn/input';

withDefaults(defineProps<{ placeholder?: string; kbd?: string }>(), { placeholder: 'بحث…' });
const model = defineModel<string>({ default: '' });
const input = ref<InstanceType<typeof Input>>();
defineExpose({
  focus: () => (input.value?.$el as HTMLInputElement | undefined)?.focus(),
  select: () => (input.value?.$el as HTMLInputElement | undefined)?.select(),
});
</script>

<template>
  <div class="relative w-full sm:w-72">
    <Search class="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-text-secondary" />
    <Input
      ref="input"
      v-model="model"
      type="search"
      :placeholder="placeholder"
      class="control h-[34px] rounded-md ps-8 pe-8 text-body shadow-none [&::-webkit-search-cancel-button]:hidden"
      @keydown.esc="model = ''"
    />
    <button
      v-if="model"
      type="button"
      aria-label="مسح"
      class="absolute end-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-text-secondary hover:text-text-primary"
      @click="model = ''; (input?.$el as HTMLInputElement | undefined)?.focus()"
    >
      <X class="size-3.5" />
    </button>
    <kbd
      v-else-if="kbd"
      class="num pointer-events-none absolute end-2 top-1/2 -translate-y-1/2 rounded border border-border px-1 text-caption text-text-secondary"
    >{{ kbd }}</kbd>
  </div>
</template>
