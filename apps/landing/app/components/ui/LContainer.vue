<script setup lang="ts">
withDefaults(defineProps<{ as?: string }>(), { as: 'div' })
</script>

<template>
  <component :is="as" class="mx-auto w-full max-w-content px-5 sm:px-8 lg:px-10">
    <slot />
  </component>
</template>
