<script setup lang="ts">
import { computed, type Component } from 'vue';
import { RouterLink, type RouteLocationRaw } from 'vue-router';
import { LoaderCircle } from '@lucide/vue';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'danger-solid';

const props = withDefaults(
  defineProps<{
    variant?: Variant;
    size?: 'sm' | 'md' | 'lg';
    type?: 'button' | 'submit';
    loading?: boolean;
    disabled?: boolean;
    icon?: Component;
    to?: RouteLocationRaw;
    block?: boolean;
    /** Keyboard hint rendered inside the button, e.g. "F12". */
    kbd?: string;
  }>(),
  { variant: 'secondary', size: 'md', type: 'button' },
);

const variantClass: Record<Variant, string> = {
  primary: 'bg-primary text-on-primary hover:bg-primary-hover border border-transparent',
  secondary: 'border border-border text-text-secondary hover:text-text-primary hover:bg-surface-hover',
  ghost: 'border border-transparent text-text-secondary hover:text-text-primary hover:bg-surface-hover',
  danger: 'border border-danger/40 text-danger hover:bg-danger/10',
  'danger-solid': 'bg-danger text-white hover:opacity-90 border border-transparent',
};

const sizeClass = {
  sm: 'h-7 px-2.5 text-xs gap-1.5',
  md: 'h-[34px] px-3.5 text-[13px] gap-2',
  lg: 'h-11 px-5 text-sm gap-2',
};

const isDisabled = computed(() => props.disabled || props.loading);
</script>

<template>
  <component
    :is="to && !isDisabled ? RouterLink : 'button'"
    :to="to && !isDisabled ? to : undefined"
    :type="to ? undefined : type"
    :disabled="to ? undefined : isDisabled"
    :aria-disabled="isDisabled || undefined"
    class="inline-flex shrink-0 select-none items-center justify-center whitespace-nowrap rounded-md font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-55"
    :class="[variantClass[variant], sizeClass[size], block && 'w-full', isDisabled && to && 'pointer-events-none opacity-55']"
  >
    <LoaderCircle v-if="loading" class="size-4 animate-spin" />
    <component :is="icon" v-else-if="icon" class="size-4 shrink-0" :stroke-width="1.75" />
    <slot />
    <kbd
      v-if="kbd"
      class="num rounded border border-current/25 px-1 font-sans text-[10px] leading-4 opacity-70"
    >{{ kbd }}</kbd>
  </component>
</template>
