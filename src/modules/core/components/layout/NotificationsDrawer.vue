<script setup lang="ts">
/**
 * Notifications drawer (docs/v2/14-platform.md §6 "Bell drawer: insights (by severity) + events").
 * A dropdown anchored to the bell icon in the topbar — mark-as-read, links to the relevant page.
 * Opens toward the page (`end-0`, like the user/branch menus) so it never runs off the left edge in
 * RTL; on narrow windows it spans the viewport under the topbar instead.
 */
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { Bell, BellRing, CheckCheck, CircleCheck } from '@lucide/vue';
import { useNotifications } from '../../controllers/useNotifications';
import { formatDateTime } from '../../helpers/format';

const router = useRouter();
const { all, unreadCount, markRead, markAllRead, isRead } = useNotifications();
const open = ref(false);
const root = ref<HTMLElement>();

const SEVERITY_CLASS: Record<string, string> = {
  critical: 'bg-danger/10 text-danger',
  warning: 'bg-warning/10 text-warning',
  info: 'bg-primary/10 text-primary',
  positive: 'bg-success/10 text-success',
};

function toggle() {
  open.value = !open.value;
}

function onOpen(n: (typeof all.value)[number]) {
  markRead(n.id);
  open.value = false;
  router.push(n.actionTo);
}

function onClickOutside(e: MouseEvent) {
  if (open.value && root.value && !root.value.contains(e.target as Node)) open.value = false;
}
function onEscape(e: KeyboardEvent) {
  if (e.key === 'Escape') open.value = false;
}
onMounted(() => {
  window.addEventListener('mousedown', onClickOutside);
  window.addEventListener('keydown', onEscape);
});
onBeforeUnmount(() => {
  window.removeEventListener('mousedown', onClickOutside);
  window.removeEventListener('keydown', onEscape);
});
</script>

<template>
  <div ref="root" class="relative">
    <button
      type="button"
      class="relative flex size-8 items-center justify-center rounded-md text-text-secondary hover:bg-surface-hover hover:text-text-primary"
      aria-label="الإشعارات"
      :aria-expanded="open"
      @click="toggle"
    >
      <BellRing v-if="unreadCount > 0" class="size-4" />
      <Bell v-else class="size-4" />
      <span
        v-if="unreadCount > 0"
        class="num text-caption absolute -end-0.5 -top-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-danger px-1 font-semibold leading-none text-white"
      >
        {{ unreadCount > 9 ? '9+' : unreadCount }}
      </span>
    </button>

    <div
      v-if="open"
      dir="rtl"
      class="fixed inset-x-2 top-14 z-50 max-h-[70vh] overflow-y-auto rounded-xl border border-border bg-background shadow-2xl sm:absolute sm:inset-x-auto sm:end-0 sm:top-full sm:mt-1.5 sm:w-96"
      role="menu"
    >
      <div class="flex items-center justify-between border-b border-border px-3.5 py-2.5">
        <p class="text-body font-semibold">الإشعارات</p>
        <button v-if="unreadCount > 0" type="button" class="flex items-center gap-1 text-tiny text-primary hover:underline" @click="markAllRead">
          <CheckCheck class="size-3.5" /> تعليم الكل كمقروء
        </button>
      </div>

      <div v-if="!all.length" class="flex flex-col items-center gap-2 px-4 py-10 text-center">
        <CircleCheck class="size-7 text-success" :stroke-width="1.5" />
        <p class="text-body text-text-secondary">لا توجد إشعارات جديدة</p>
      </div>

      <ul v-else class="divide-y divide-border">
        <li v-for="n in all" :key="n.id">
          <button
            type="button"
            class="flex w-full items-start gap-2.5 px-3.5 py-2.5 text-start hover:bg-surface-hover"
            :class="!isRead(n.id) && 'bg-primary/5'"
            @click="onOpen(n)"
          >
            <span class="flex size-7 shrink-0 items-center justify-center rounded-lg" :class="SEVERITY_CLASS[n.severity]">
              <component :is="n.icon" class="size-3.5" :stroke-width="1.75" />
            </span>
            <span class="min-w-0 flex-1">
              <span class="block text-body leading-5" :class="!isRead(n.id) && 'font-medium'">{{ n.message }}</span>
              <span class="num mt-0.5 block text-tiny text-text-secondary">{{ formatDateTime(n.createdAt) }}</span>
            </span>
            <span v-if="!isRead(n.id)" class="mt-1.5 size-1.5 shrink-0 rounded-full bg-primary" />
          </button>
        </li>
      </ul>
    </div>
  </div>
</template>
