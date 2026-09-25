<script setup lang="ts">
/**
 * Notifications drawer (docs/v2/14-platform.md §6 "Bell drawer: insights (by severity) + events"),
 * rebuilt on shadcn's Sheet (docs/v2/17-ui-system-rtl-themes.md Phase D) — opens from the side
 * closest to the bell (the topbar's trailing/start edge in RTL, i.e. physically left) rather than
 * the hand-rolled dropdown it used to be. `useNotifications()`'s composable API is unchanged.
 */
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { Bell, BellRing, CheckCheck, CircleCheck } from '@lucide/vue';
import { useNotifications } from '../../controllers/useNotifications';
import { formatDateTime } from '../../helpers/format';
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/modules/core/components/shadcn/sheet';

const router = useRouter();
const { all, unreadCount, markRead, markAllRead, isRead } = useNotifications();
const open = ref(false);

const SEVERITY_CLASS: Record<string, string> = {
  critical: 'bg-danger/10 text-danger',
  warning: 'bg-warning/10 text-warning',
  info: 'bg-primary/10 text-primary',
  positive: 'bg-success/10 text-success',
};

function onOpen(n: (typeof all.value)[number]) {
  markRead(n.id);
  open.value = false;
  router.push(n.actionTo);
}
</script>

<template>
  <Sheet v-model:open="open">
    <button
      type="button"
      class="relative flex size-8 items-center justify-center rounded-md text-text-secondary hover:bg-surface-hover hover:text-text-primary"
      aria-label="الإشعارات"
      @click="open = true"
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

    <!-- rtl-ok: the sheet opens from the left, the side toward the topbar's start edge in RTL -->
    <SheetContent side="left" dir="rtl" class="flex w-full flex-col gap-0 p-0 sm:max-w-sm">
      <SheetHeader class="flex-row items-center justify-between space-y-0 border-b border-border">
        <SheetTitle>الإشعارات</SheetTitle>
        <button v-if="unreadCount > 0" type="button" class="flex items-center gap-1 text-tiny text-primary hover:underline" @click="markAllRead">
          <CheckCheck class="size-3.5" /> تعليم الكل كمقروء
        </button>
      </SheetHeader>

      <div v-if="!all.length" class="flex flex-1 flex-col items-center justify-center gap-2 px-4 text-center">
        <CircleCheck class="size-7 text-success" :stroke-width="1.5" />
        <p class="text-body text-text-secondary">لا توجد إشعارات جديدة</p>
      </div>

      <ul v-else class="flex-1 divide-y divide-border overflow-y-auto">
        <li v-for="n in all" :key="n.id">
          <button
            type="button"
            class="flex w-full items-start gap-2.5 px-4 py-2.5 text-start hover:bg-surface-hover"
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
    </SheetContent>
  </Sheet>
</template>
