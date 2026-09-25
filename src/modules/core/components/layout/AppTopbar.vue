<script setup lang="ts">
import { onMounted } from 'vue';
import { Moon, Search, ShoppingCart, Sun } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useBranchStore } from '@/modules/settings/controllers/useBranchStore';
import { useCommandPalette } from '../../controllers/useCommandPalette';
import { resolvedTheme, toggleTheme } from '../../controllers/useTheme';
import AppButton from '../ui/AppButton.vue';
import DevMenu from '../DevMenu.vue';
import BranchSwitcher from './BranchSwitcher.vue';
import NotificationsDrawer from './NotificationsDrawer.vue';
import UserMenu from './UserMenu.vue';

const palette = useCommandPalette();

const auth = useAuthStore();
const branchStore = useBranchStore();
const isDev = import.meta.env.DEV;

onMounted(() => {
  branchStore.load();
});
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
    <DevMenu v-if="isDev" />
    <BranchSwitcher v-if="branchStore.showSwitcher" />
    <AppButton v-if="auth.can('pos', 'write')" to="/pos" size="sm" :icon="ShoppingCart">نقطة البيع</AppButton>
    <NotificationsDrawer />
    <button
      type="button"
      class="flex size-8 items-center justify-center rounded-md text-text-secondary hover:bg-surface-hover hover:text-text-primary"
      :aria-label="resolvedTheme === 'dark' ? 'الوضع الفاتح' : 'الوضع الداكن'"
      :title="resolvedTheme === 'dark' ? 'الوضع الفاتح' : 'الوضع الداكن'"
      @click="toggleTheme"
    >
      <Sun v-if="resolvedTheme === 'dark'" class="size-4" />
      <Moon v-else class="size-4" />
    </button>
    <div class="mx-1 h-5 w-px bg-border" />
    <UserMenu />
  </div>
</template>
