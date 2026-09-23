<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRoute } from 'vue-router';
import { Moon, Search, ShoppingCart, Sun } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useBranchStore } from '@/modules/settings/controllers/useBranchStore';
import { useCommandPalette } from '../../controllers/useCommandPalette';
import { resolvedTheme, toggleTheme } from '../../controllers/useTheme';
import AppButton from '../ui/AppButton.vue';
import DevMenu from '../DevMenu.vue';
import BranchSwitcher from './BranchSwitcher.vue';
import UserMenu from './UserMenu.vue';

const palette = useCommandPalette();

const route = useRoute();
const auth = useAuthStore();
const branchStore = useBranchStore();
const isDev = import.meta.env.DEV;

onMounted(() => {
  branchStore.load();
});

const crumbs = computed(() => {
  const list: string[] = [];
  if (route.meta.section) list.push(route.meta.section);
  if (route.meta.title) list.push(route.meta.title);
  return list;
});
</script>

<template>
  <header class="no-print flex h-[52px] shrink-0 items-center justify-between gap-4 border-b border-border bg-background px-5">
    <nav aria-label="مسار التنقل" class="flex min-w-0 items-center gap-1.5 text-body">
      <template v-for="(c, i) in crumbs" :key="i">
        <span v-if="i > 0" class="text-text-secondary/60">/</span>
        <span class="truncate" :class="i === crumbs.length - 1 ? 'font-medium' : 'text-text-secondary'">{{ c }}</span>
      </template>
    </nav>
    <div class="flex items-center gap-1.5">
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
  </header>
</template>
