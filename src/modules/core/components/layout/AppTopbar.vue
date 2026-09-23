<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { Moon, ShoppingCart, Sun } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { resolvedTheme, toggleTheme } from '../../controllers/useTheme';
import AppButton from '../ui/AppButton.vue';
import DevMenu from '../DevMenu.vue';
import UserMenu from './UserMenu.vue';

const route = useRoute();
const auth = useAuthStore();
const isDev = import.meta.env.DEV;

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
      <DevMenu v-if="isDev" />
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
