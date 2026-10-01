<script setup lang="ts">
import { ConfigProvider } from "reka-ui";
import { Toaster } from "@/shared/components/ui/sonner";
import { ConfirmDialog } from "@/shared/components/app";
</script>

<template>
  <ConfigProvider dir="rtl" locale="ar">
    <router-view />
    <ConfirmDialog />
    <Toaster dir="rtl" position="top-center" rich-colors />
  </ConfigProvider>
</template>
