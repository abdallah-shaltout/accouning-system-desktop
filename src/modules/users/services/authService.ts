import { ApiError, clone, db, delay, session } from '@/mocks';
import { logActivity } from '@/mocks/backend/core';
import type { User } from '../types';

/** Mock auth: checks fixture credentials, no hashing — a real IPC `login` command replaces this. */
export async function login(username: string, password: string): Promise<User> {
  await delay(450);
  const user = db.users.find((u) => u.username.toLowerCase() === username.trim().toLowerCase());
  if (!user || db.credentials[user.username] !== password) {
    throw new ApiError('اسم المستخدم أو كلمة المرور غير صحيحة', 'UNAUTHORIZED');
  }
  if (!user.active) throw new ApiError('هذا الحساب موقوف — تواصل مع مدير النظام', 'FORBIDDEN');
  session.userId = user.id;
  logActivity('auth', `تسجيل دخول ${user.name}`, user.id, new Date().toISOString());
  return clone(user);
}

/** Re-attach a session after reload (the id is kept in localStorage by the auth store). */
export async function restoreSession(userId: string): Promise<User | null> {
  await delay(60);
  const user = db.users.find((u) => u.id === userId && u.active);
  if (!user) return null;
  session.userId = user.id;
  return clone(user);
}

export async function logout(): Promise<void> {
  await delay(80);
  session.userId = '';
}

/** Demo accounts listed on the login screen (mock only). */
export async function getDemoAccounts(): Promise<{ id: string; username: string; password: string; name: string; role: User['role'] }[]> {
  await delay(50);
  return db.users
    .filter((u) => u.active)
    .map((u) => ({ id: u.id, username: u.username, password: db.credentials[u.username], name: u.name, role: u.role }));
}
