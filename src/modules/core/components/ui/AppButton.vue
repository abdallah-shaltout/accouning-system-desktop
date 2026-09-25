<script setup lang="ts">
/**
 * Rebuilt on shadcn's Button (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — same props/emits API
 * as before, so none of this component's 100+ call sites needed to change. Sizes keep their exact
 * previous heights (h-7/h-[34px]/h-11) rather than switching to shadcn's own sm/default/lg scale, to
 * avoid a layout shift across every page that weren't part of this rebrand.
 */
import { computed, type Component } from 'vue';
import { RouterLink, type RouteLocationRaw } from 'vue-router';
import { LoaderCircle } from '@lucide/vue';
import { Button } from '@/modules/core/components/shadcn/button';
import type { ButtonVariants } from '@/modules/core/components/shadcn/button';

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

// shadcn's own variants don't distinguish our bordered "secondary"/"ghost"/outlined "danger" from
// its filled ones, so extra utility classes below cover the gap on top of its base variant.
const shadcnVariant: Record<Variant, NonNullable<ButtonVariants['variant']>> = {
  primary: 'default',
  secondary: 'outline',
  ghost: 'ghost',
  danger: 'outline',
  'danger-solid': 'destructive',
};

const extraClass: Record<Variant, string> = {
  primary: '',
  secondary: 'text-text-secondary hover:text-text-primary',
  ghost: 'border-transparent text-text-secondary hover:text-text-primary',
  danger: 'border-danger/40 text-danger hover:bg-danger/10 hover:text-danger',
  'danger-solid': '',
};

const sizeClass = {
  sm: 'h-7 px-2.5 text-xs gap-1.5',
  md: 'h-[34px] px-3.5 text-body gap-2',
  lg: 'h-11 px-5 text-sm gap-2',
};

const isDisabled = computed(() => props.disabled || props.loading);
</script>

<template>
  <Button
    :as="to && !isDisabled ? RouterLink : 'button'"
    :to="to && !isDisabled ? to : undefined"
    :type="to ? undefined : type"
    :disabled="to ? undefined : isDisabled"
    :aria-disabled="isDisabled || undefined"
    :variant="shadcnVariant[variant]"
    class="rounded-md font-medium"
    :class="[extraClass[variant], sizeClass[size], block && 'w-full', isDisabled && to && 'pointer-events-none opacity-55']"
  >
    <LoaderCircle v-if="loading" class="size-4 animate-spin" />
    <component :is="icon" v-else-if="icon" class="size-4 shrink-0" :stroke-width="1.75" />
    <slot />
    <kbd
      v-if="kbd"
      class="num rounded border border-current/25 px-1 font-sans text-caption leading-4 opacity-70"
    >{{ kbd }}</kbd>
  </Button>
</template>
