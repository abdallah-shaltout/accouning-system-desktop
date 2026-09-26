<script setup lang="ts">
/**
 * docs/v2/17-ui-system-rtl-themes.md Phase D — sidebar footer: initials avatar, name, role; a
 * shadcn DropdownMenu with المظهر، الوضع الداكن، اختصارات لوحة المفاتيح، الإعدادات، (dev: تبديل
 * المستخدم)، تسجيل الخروج. Replaces the old topbar UserMenu.vue; DevMenu's reset/reload-demo/
 * latency controls move here too (the dev section), and the theme toggle moves here from the topbar.
 */
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { Bug, ChevronsUpDown, DatabaseBackup, Keyboard, LogOut, Moon, Palette, RefreshCw, Sun, UserRound } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getDemoAccounts } from '@/modules/users/services/authService';
import { useBackupStore } from '@/modules/settings/controllers/useBackupStore';
import { formatDateTime } from '../../helpers/format';
import { ROLE_LABEL } from '../../helpers/labels';
import { useToast } from '../../controllers/useToast';
import { useConfirm } from '../../controllers/useConfirm';
import { resolvedTheme, toggleTheme } from '../../controllers/useTheme';
import { useKeyboardShortcutsSheet } from '../../controllers/useKeyboardShortcutsSheet';
import { seedDatabase } from '@/mocks/seed';
import { clearSnapshot, flushSnapshot } from '@/mocks/persist';
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from '@/modules/core/components/shadcn/sidebar';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/modules/core/components/shadcn/dropdown-menu';

const auth = useAuthStore();
const router = useRouter();
const toast = useToast();
const askConfirm = useConfirm();
const backup = useBackupStore();
const { isMobile } = useSidebar();
const shortcutsSheet = useKeyboardShortcutsSheet();
const isDev = import.meta.env.DEV;
const demo = ref<Awaited<ReturnType<typeof getDemoAccounts>>>([]);
const busy = ref(false);

const initials = computed(() => (auth.user?.name ?? '?').trim().split(/\s+/).slice(0, 2).map((w) => w[0]).join(''));

async function onOpen(open: boolean) {
  if (!open) return;
  if (isDev && !demo.value.length) demo.value = await getDemoAccounts();
  await backup.load();
}

async function switchUser(account: (typeof demo.value)[number]) {
  await auth.switchTo(account.id);
  toast.info(`تم التبديل إلى ${account.name}`, ROLE_LABEL[account.role]);
  await router.replace({ path: router.currentRoute.value.fullPath, force: true }); /* route-ok: forces a reload of the current URL after switching accounts, not a navigation target */
}

async function logout() {
  await auth.logout();
  router.push({ name: 'login' });
}

async function resetData() {
  const ok = await askConfirm({
    title: 'إعادة تعيين البيانات',
    message: 'سيتم حذف كل البيانات المحفوظة محلياً والعودة لشاشة البداية. هذا الإجراء لا يمكن التراجع عنه.',
    confirmText: 'إعادة التعيين',
    danger: true,
  });
  if (!ok) return;
  busy.value = true;
  try {
    await clearSnapshot();
    window.location.reload();
  } finally {
    busy.value = false;
  }
}

async function reloadDemoData() {
  const ok = await askConfirm({
    title: 'تحميل البيانات التجريبية',
    message: 'سيتم استبدال كل البيانات الحالية ببيانات تجريبية جديدة.',
    confirmText: 'تحميل',
    danger: true,
  });
  if (!ok) return;
  busy.value = true;
  try {
    seedDatabase();
    await flushSnapshot();
    toast.info('تم تحميل البيانات التجريبية');
    window.location.reload();
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <SidebarMenu>
    <SidebarMenuItem>
      <DropdownMenu @update:open="onOpen">
        <DropdownMenuTrigger as-child>
          <SidebarMenuButton size="lg" class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground">
            <span class="flex size-8 shrink-0 items-center justify-center rounded-full bg-primary/15 text-xs font-semibold text-primary">{{ initials }}</span>
            <div class="grid min-w-0 flex-1 text-start leading-tight">
              <span class="truncate font-medium">{{ auth.user?.name }}</span>
              <span class="truncate text-xs text-text-secondary">{{ auth.role ? ROLE_LABEL[auth.role] : '' }}</span>
            </div>
            <ChevronsUpDown class="ms-auto size-4" />
          </SidebarMenuButton>
        </DropdownMenuTrigger>
        <DropdownMenuContent class="w-64" :side="isMobile ? 'bottom' : 'right'" align="end" :side-offset="4">
          <DropdownMenuLabel class="p-0 font-normal">
            <div class="px-1 py-1.5">
              <p class="text-body font-medium">{{ auth.user?.name }}</p>
              <p class="text-xs text-text-secondary"><span class="num">@{{ auth.user?.username }}</span> · {{ auth.role ? ROLE_LABEL[auth.role] : '' }}</p>
            </div>
          </DropdownMenuLabel>
          <DropdownMenuSeparator v-if="auth.can('settings')" />
          <DropdownMenuItem v-if="auth.can('settings')" as-child>
            <RouterLink :to="{ name: 'settings-backup' }" class="flex w-full items-center gap-2">
              <DatabaseBackup class="size-4" />
              <span v-if="backup.lastBackupAt">آخر نسخة احتياطية: <span class="num">{{ formatDateTime(backup.lastBackupAt) }}</span></span>
              <span v-else>لا توجد نسخة احتياطية بعد</span>
            </RouterLink>
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem as-child>
            <RouterLink :to="{ name: 'settings-appearance' }" class="flex w-full items-center gap-2">
              <Palette class="size-4" />
              <span>المظهر والأرقام</span>
            </RouterLink>
          </DropdownMenuItem>
          <DropdownMenuItem @select.prevent="toggleTheme">
            <Sun v-if="resolvedTheme === 'dark'" class="size-4" />
            <Moon v-else class="size-4" />
            <span>{{ resolvedTheme === 'dark' ? 'الوضع الفاتح' : 'الوضع الداكن' }}</span>
          </DropdownMenuItem>
          <DropdownMenuItem @select="shortcutsSheet.show()">
            <Keyboard class="size-4" />
            <span>اختصارات لوحة المفاتيح (F1)</span>
          </DropdownMenuItem>
          <template v-if="isDev">
            <DropdownMenuSeparator />
            <DropdownMenuLabel class="flex items-center gap-1.5 text-tiny font-normal text-text-secondary">
              <RefreshCw class="size-3" /> تبديل المستخدم (وضع التطوير)
            </DropdownMenuLabel>
            <DropdownMenuItem
              v-for="d in demo"
              :key="d.username"
              :disabled="d.username === auth.user?.username"
              @select="switchUser(d)"
            >
              <UserRound class="size-4" />
              <span class="flex-1 truncate">{{ d.name }}</span>
              <span class="text-tiny text-text-secondary">{{ ROLE_LABEL[d.role] }}</span>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem as-child>
              <RouterLink :to="{ name: 'dev-diagnostics' }" class="flex w-full items-center gap-2">
                <Bug class="size-4" />
                <span>التشخيص</span>
              </RouterLink>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem :disabled="busy" @select="reloadDemoData">
              <RefreshCw class="size-4" />
              <span>تحميل بيانات تجريبية جديدة</span>
            </DropdownMenuItem>
            <DropdownMenuItem variant="destructive" :disabled="busy" @select="resetData">
              <RefreshCw class="size-4" />
              <span>إعادة تعيين البيانات</span>
            </DropdownMenuItem>
          </template>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive" @select="logout">
            <LogOut class="size-4" />
            <span>تسجيل الخروج</span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </SidebarMenuItem>
  </SidebarMenu>
</template>
