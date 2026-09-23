<script setup lang="ts">
import CommandPalette from '@/modules/core/components/CommandPalette.vue';
import ToastContainer from '@/modules/core/components/ToastContainer.vue';
import ConfirmDialog from '@/modules/core/components/ui/ConfirmDialog.vue';
</script>

<template>
  <RouterView />
  <ToastContainer />
  <ConfirmDialog />
  <CommandPalette />
</template>
