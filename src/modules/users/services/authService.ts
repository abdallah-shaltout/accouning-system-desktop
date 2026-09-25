import { ApiError, clone, db, delay, session } from '@/mocks';
import { logActivity } from '@/mocks/backend/core';
import type { User } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';

/** Mock auth: checks fixture credentials, no hashing — a real IPC `login` command replaces this. */
export const login = wrap('users.login', async function login(username: string, password: string): Promise<User> {
  await delay(450);
  const user = db.users.find((u) => u.username.toLowerCase() === username.trim().toLowerCase());
  if (!user || db.credentials[user.username] !== password) {
    throw new ApiError('اسم المستخدم أو كلمة المرور غير صحيحة', 'UNAUTHORIZED');
  }
  if (!user.active) throw new ApiError('هذا الحساب موقوف — تواصل مع مدير النظام', 'FORBIDDEN');
  session.userId = user.id;
  logActivity('auth', `تسجيل دخول ${user.name}`, user.id, new Date().toISOString());
  return clone(user);
});

/** Re-attach a session after reload (the id is kept in localStorage by the auth store). */
export const restoreSession = wrap('users.restoreSession', async function restoreSession(userId: string): Promise<User | null> {
  await delay(60);
  const user = db.users.find((u) => u.id === userId && u.active);
  if (!user) return null;
  session.userId = user.id;
  return clone(user);
});

export const logout = wrap('users.logout', async function logout(): Promise<void> {
  await delay(80);
  session.userId = '';
});

/**
 * v2 phase 6 §5 (docs/v2/07-products-and-inventory.md, docs/v2/01-personas.md storekeeper): checks
 * a manager/admin's password WITHOUT switching the active session — used by the inventory
 * approval-threshold PIN dialog (a stock-in/write-off above the configured value needs a manager to
 * type their own password to approve it, then the original user's session continues unchanged).
 * Returns the approving user on success so the caller can stamp `approvedBy`.
 */
export const verifyManagerPin = wrap('users.verifyManagerPin', async function verifyManagerPin(username: string, password: string): Promise<User> {
  await delay(300);
  const user = db.users.find((u) => u.username.toLowerCase() === username.trim().toLowerCase());
  if (!user || db.credentials[user.username] !== password) {
    throw new ApiError('اسم المستخدم أو كلمة المرور غير صحيحة', 'UNAUTHORIZED');
  }
  if (!user.active) throw new ApiError('هذا الحساب موقوف', 'FORBIDDEN');
  if (user.role !== 'admin' && user.role !== 'manager') throw new ApiError('هذا المستخدم ليس مديراً — الاعتماد يتطلب صلاحية مدير', 'FORBIDDEN');
  return clone(user);
});

/** Demo accounts listed on the login screen (mock only). */
export const getDemoAccounts = wrap('users.getDemoAccounts', async function getDemoAccounts(): Promise<{ id: string; username: string; password: string; name: string; role: User['role'] }[]> {
  await delay(50);
  return db.users
    .filter((u) => u.active)
    .map((u) => ({ id: u.id, username: u.username, password: db.credentials[u.username], name: u.name, role: u.role }));
});
