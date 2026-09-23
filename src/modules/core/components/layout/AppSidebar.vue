<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { PanelRightClose, PanelRightOpen, Store } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { NAVIGATION, type NavItem } from '../../helpers/navigation';

const collapsed = defineModel<boolean>('collapsed', { default: false });

const route = useRoute();
const auth = useAuthStore();
const settings = useSettingsStore();

const groups = computed(() =>
  NAVIGATION.map((g) => ({ ...g, items: g.items.filter((i) => auth.can(i.area)) })).filter((g) => g.items.length),
);

function isActive(item: NavItem) {
  if (item.exact) return route.path === item.to;
  return route.path === item.to || route.path.startsWith(`${item.to}/`);
}
</script>

<template>
  <aside
    class="no-print flex h-full shrink-0 flex-col border-e border-border bg-surface transition-[width] duration-200"
    :class="collapsed ? 'w-[60px]' : 'w-[232px]'"
  >
    <div class="flex h-[52px] items-center gap-2.5 border-b border-border px-3.5">
      <img
        v-if="settings.settings?.logo"
        :src="settings.settings.logo"
        alt=""
        class="size-7 shrink-0 rounded-md border border-border bg-background object-contain"
      />
      <div v-else class="flex size-7 shrink-0 items-center justify-center rounded-md bg-primary text-on-primary">
        <Store class="size-4" />
      </div>
      <div v-if="!collapsed" class="min-w-0">
        <p class="truncate text-body font-semibold">{{ settings.settings?.storeName ?? '—' }}</p>
        <p class="truncate text-tiny text-text-secondary">المحاسبة ونقاط البيع</p>
      </div>
    </div>

    <nav class="flex-1 overflow-y-auto overflow-x-hidden px-2 py-3" aria-label="القائمة الرئيسية">
      <div v-for="(group, gi) in groups" :key="gi" class="mb-3 last:mb-0">
        <p v-if="group.label && !collapsed" class="mb-1 px-2.5 text-tiny font-medium text-text-secondary/80">{{ group.label }}</p>
        <div v-else-if="group.label" class="mx-2.5 mb-2 border-t border-border" />
        <RouterLink
          v-for="item in group.items"
          :key="item.to"
          :to="item.to"
          :title="collapsed ? item.label : undefined"
          class="relative mb-0.5 flex h-8 items-center gap-2.5 rounded-md px-2.5 text-body transition-colors"
          :class="
            isActive(item)
              ? 'bg-background font-medium text-primary shadow-[inset_0_0_0_1px_var(--color-border)]'
              : 'text-text-secondary hover:bg-surface-hover hover:text-text-primary'
          "
        >
          <span v-if="isActive(item)" class="absolute inset-y-1.5 start-0 w-0.5 rounded-full bg-primary" />
          <component :is="item.icon" class="size-4 shrink-0" :stroke-width="1.75" />
          <span v-if="!collapsed" class="truncate">{{ item.label }}</span>
        </RouterLink>
      </div>
    </nav>

    <button
      type="button"
      class="flex h-10 items-center gap-2.5 border-t border-border px-4 text-xs text-text-secondary hover:bg-surface-hover hover:text-text-primary"
      :aria-label="collapsed ? 'توسيع القائمة' : 'طي القائمة'"
      @click="collapsed = !collapsed"
    >
      <PanelRightOpen v-if="collapsed" class="size-4" />
      <PanelRightClose v-else class="size-4" />
      <span v-if="!collapsed">طي القائمة</span>
    </button>
  </aside>
</template>
