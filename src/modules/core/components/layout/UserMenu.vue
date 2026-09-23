<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import { useRouter } from 'vue-router';
import { ChevronDown, DatabaseBackup, LogOut, Palette, RefreshCw, UserRound } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getDemoAccounts } from '@/modules/users/services/authService';
import { useBackupStore } from '@/modules/settings/controllers/useBackupStore';
import { formatDateTime } from '../../helpers/format';
import { ROLE_LABEL } from '../../helpers/labels';
import { useToast } from '../../controllers/useToast';

const auth = useAuthStore();
const router = useRouter();
const toast = useToast();
const backup = useBackupStore();
const open = ref(false);
const root = ref<HTMLElement>();
const demo = ref<Awaited<ReturnType<typeof getDemoAccounts>>>([]);
const isDev = import.meta.env.DEV;

const initials = computed(() => (auth.user?.name ?? '?').trim().split(/\s+/).slice(0, 2).map((w) => w[0]).join(''));

async function toggle() {
  open.value = !open.value;
  if (open.value) {
    document.addEventListener('mousedown', onOutside);
    if (isDev && !demo.value.length) demo.value = await getDemoAccounts();
    await backup.load();
  } else document.removeEventListener('mousedown', onOutside);
}

function onOutside(e: MouseEvent) {
  if (!root.value?.contains(e.target as Node)) {
    open.value = false;
    document.removeEventListener('mousedown', onOutside);
  }
}

async function switchUser(account: (typeof demo.value)[number]) {
  await auth.switchTo(account.id);
  open.value = false;
  toast.info(`تم التبديل إلى ${account.name}`, ROLE_LABEL[account.role]);
  // Re-run route guards: the new role may not be allowed on the current page.
  await router.replace({ path: router.currentRoute.value.fullPath, force: true });
}

async function logout() {
  open.value = false;
  await auth.logout();
  router.push({ name: 'login' });
}

onBeforeUnmount(() => document.removeEventListener('mousedown', onOutside));
</script>

<template>
  <div ref="root" class="relative">
    <button
      type="button"
      class="flex h-8 items-center gap-2 rounded-md px-1.5 hover:bg-surface-hover"
      :aria-expanded="open"
      aria-haspopup="menu"
      @click="toggle"
    >
      <span class="flex size-7 items-center justify-center rounded-full bg-primary/15 text-xs font-semibold text-primary">{{ initials }}</span>
      <span class="hidden text-start leading-tight md:block">
        <span class="block text-body">{{ auth.user?.name }}</span>
        <span class="block text-tiny text-text-secondary">{{ auth.role ? ROLE_LABEL[auth.role] : '' }}</span>
      </span>
      <ChevronDown class="size-3.5 text-text-secondary" />
    </button>

    <div
      v-if="open"
      role="menu"
      class="absolute end-0 top-full z-50 mt-1.5 w-64 overflow-hidden rounded-lg border border-border bg-background shadow-xl"
    >
      <div class="border-b border-border px-3 py-2.5">
        <p class="text-body font-medium">{{ auth.user?.name }}</p>
        <p class="text-xs text-text-secondary">
          <span class="num">@{{ auth.user?.username }}</span> · {{ auth.role ? ROLE_LABEL[auth.role] : '' }}
        </p>
      </div>
      <div v-if="isDev && demo.length" class="border-b border-border p-1">
        <p class="flex items-center gap-1.5 px-2 py-1 text-tiny text-text-secondary">
          <RefreshCw class="size-3" /> تبديل المستخدم (وضع التطوير)
        </p>
        <button
          v-for="d in demo"
          :key="d.username"
          type="button"
          role="menuitem"
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-start text-body hover:bg-surface-hover disabled:opacity-50"
          :disabled="d.username === auth.user?.username"
          @click="switchUser(d)"
        >
          <UserRound class="size-3.5 text-text-secondary" />
          <span class="flex-1 truncate">{{ d.name }}</span>
          <span class="text-tiny text-text-secondary">{{ ROLE_LABEL[d.role] }}</span>
        </button>
      </div>
      <div v-if="auth.can('settings')" class="border-b border-border px-3 py-2 text-tiny text-text-secondary">
        <RouterLink to="/settings/backup" class="flex items-center gap-1.5 hover:text-text-primary" @click="open = false">
          <DatabaseBackup class="size-3" />
          <span v-if="backup.lastBackupAt">آخر نسخة احتياطية: <span class="num">{{ formatDateTime(backup.lastBackupAt) }}</span></span>
          <span v-else>لا توجد نسخة احتياطية بعد</span>
        </RouterLink>
      </div>
      <div class="p-1">
        <RouterLink
          to="/settings/appearance"
          role="menuitem"
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-body hover:bg-surface-hover"
          @click="open = false"
        >
          <Palette class="size-3.5 text-text-secondary" /> المظهر والأرقام
        </RouterLink>
        <button
          type="button"
          role="menuitem"
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-start text-body text-danger hover:bg-danger/10"
          @click="logout"
        >
          <LogOut class="size-3.5" /> تسجيل الخروج
        </button>
      </div>
    </div>
  </div>
</template>
