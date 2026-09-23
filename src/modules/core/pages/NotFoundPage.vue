<script setup lang="ts">
import { House, SearchX } from '@lucide/vue';
import AppButton from '../components/ui/AppButton.vue';
import EmptyState from '../components/ui/EmptyState.vue';
</script>

<template>
  <EmptyState :icon="SearchX" title="الصفحة غير موجودة" description="ربما تم نقلها أو أن الرابط غير صحيح.">
    <AppButton :icon="House" to="/">العودة للرئيسية</AppButton>
  </EmptyState>
</template>
