<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import type { Component } from 'vue';
import { cn } from '@/modules/core/helpers/utils';

defineProps<{
  icon: Component;
  class?: HTMLAttributes['class'];
}>();
</script>

<template>
  <component :is="icon" :class="cn('rtl:-scale-x-100', $props.class)" />
</template>
