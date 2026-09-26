<script setup lang="ts">
import { Search, ShoppingCart } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useCommandPalette } from '../../controllers/useCommandPalette';
import AppButton from '../ui/AppButton.vue';
import NotificationsDrawer from './NotificationsDrawer.vue';

const palette = useCommandPalette();
const auth = useAuthStore();
</script>

<template>
  <div class="flex min-w-0 flex-1 items-center justify-end gap-1.5">
    <button
      type="button"
      class="hidden items-center gap-2 rounded-md border border-border px-2.5 py-1.5 text-xs text-text-secondary hover:bg-surface-hover hover:text-text-primary sm:flex"
      @click="palette.show()"
    >
      <Search class="size-3.5" />
      ابحث أو نفّذ أمراً…
      <kbd class="num rounded border border-border px-1">Ctrl K</kbd>
    </button>
    <AppButton v-if="auth.can('pos', 'write')" :to="{ name: 'pos' }" size="sm" :icon="ShoppingCart">نقطة البيع</AppButton>
    <NotificationsDrawer />
  </div>
</template>
