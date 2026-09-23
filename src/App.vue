<script setup lang="ts">
import ToastContainer from '@/modules/core/components/ToastContainer.vue';
import ConfirmDialog from '@/modules/core/components/ui/ConfirmDialog.vue';
</script>

<template>
  <RouterView />
  <ToastContainer />
  <ConfirmDialog />
</template>
