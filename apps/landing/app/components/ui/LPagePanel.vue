<script setup lang="ts">
/** White rounded page panel on the forest frame, used by every inner page (blog, legal). */
</script>

<template>
  <div class="px-2.5">
    <div class="rounded-b-panel bg-white pt-32 pb-20 lg:pt-44 lg:pb-28">
      <slot />
    </div>
  </div>
</template>
