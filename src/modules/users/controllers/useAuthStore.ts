import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as authService from '../services/authService';
import { roleCan, roleCanRestoreBackup } from '../helpers/permissions';
import type { Access, Area, User } from '../types';

const SESSION_KEY = 'app_session_user';

function readSession(): string | null {
  try {
    return localStorage.getItem(SESSION_KEY);
  } catch {
    return null;
  }
}

function writeSession(userId: string | null) {
  try {
    if (userId) localStorage.setItem(SESSION_KEY, userId);
    else localStorage.removeItem(SESSION_KEY);
  } catch {
    /* ignore */
  }
}

/** The current (mock) session user. Only the user id is persisted, as a UI convenience. */
export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null);
  const restored = ref(false);

  const isAuthenticated = computed(() => !!user.value);
  const role = computed(() => user.value?.role);

  function can(area: Area, access: Exclude<Access, 'none'> = 'read'): boolean {
    return roleCan(role.value, area, access);
  }

  /** `settings.restoreBackup` — see `roleCanRestoreBackup` for why this maps to settings:write. */
  const canRestoreBackup = computed(() => roleCanRestoreBackup(role.value));

  async function login(username: string, password: string) {
    user.value = await authService.login(username, password);
    writeSession(user.value.id);
  }

  /** Called once by the router guard before the first navigation. */
  async function restore() {
    if (restored.value) return;
    restored.value = true;
    const id = readSession();
    if (!id) return;
    user.value = await authService.restoreSession(id);
    if (!user.value) writeSession(null);
  }

  async function logout() {
    await authService.logout();
    user.value = null;
    writeSession(null);
  }

  /** Dev-only quick switch between demo users (no password prompt). */
  async function switchTo(userId: string) {
    const next = await authService.restoreSession(userId);
    if (next) {
      user.value = next;
      writeSession(next.id);
    }
  }

  /** Keep the header/avatar in sync after the current user edits their own profile. */
  function patchCurrent(updated: User) {
    if (user.value?.id === updated.id) user.value = updated;
  }

  return { user, role, isAuthenticated, can, canRestoreBackup, login, restore, logout, switchTo, patchCurrent };
});
