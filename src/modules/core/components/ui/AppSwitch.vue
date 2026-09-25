<script setup lang="ts">
/** Rebuilt on shadcn's Switch, i.e. reka-ui's SwitchRoot/SwitchThumb (docs/v2/16-equal-rebrand-and-
 * ui-kit.md Phase C) — same props as before. Unlike a plain input/select, this is a real
 * accessibility upgrade over the old hand-rolled `<button role="switch">`: reka-ui handles
 * Space/Enter toggling and focus state itself instead of a manual `@click` handler. */
import { Switch } from '@/modules/core/components/shadcn/switch';

defineProps<{ label?: string; description?: string; disabled?: boolean }>();
const model = defineModel<boolean>({ default: false });
</script>

<template>
  <label class="flex cursor-pointer items-start gap-3" :class="disabled && 'cursor-not-allowed opacity-60'">
    <Switch
      v-model="model"
      :disabled="disabled"
      class="mt-0.5 h-[1.15rem] w-8 border border-border-control data-[state=checked]:border-primary data-[state=checked]:bg-primary data-[state=unchecked]:bg-surface-hover"
    >
      <template #thumb>
        <span class="pointer-events-none block size-4 rounded-full bg-white shadow-sm" />
      </template>
    </Switch>
    <span v-if="label || description" class="min-w-0">
      <span class="block text-body text-text-primary">{{ label }}</span>
      <span v-if="description" class="block text-xs text-text-secondary">{{ description }}</span>
    </span>
  </label>
</template>
