import { ApiError, clone, db, delay, session, uid } from '@/mocks';
import { logActivity } from '@/mocks/backend/core';
import { mutate } from '@/mocks/persist';
import type { User, UserInput } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';
import { backendCall, usesRust } from '@/modules/core/services/backend';

export const getUsers = wrap('users.getUsers', async function getUsers(): Promise<User[]> {
  if (usesRust('users')) return backendCall('users_get_users');
  await delay();
  return clone(db.users);
});

export const getUser = wrap('users.getUser', async function getUser(id: string): Promise<User> {
  if (usesRust('users')) return backendCall('users_get_user', { id });
  await delay();
  const user = db.users.find((u) => u.id === id);
  if (!user) throw new ApiError('المستخدم غير موجود', 'NOT_FOUND');
  return clone(user);
});

function assertUnique(username: string, exceptId?: string) {
  if (db.users.some((u) => u.id !== exceptId && u.username.toLowerCase() === username.toLowerCase())) {
    throw new ApiError('اسم المستخدم مستخدم من قبل', 'CONFLICT');
  }
}

export const createUser = wrap('users.createUser', async function createUser(input: UserInput): Promise<User> {
  if (usesRust('users')) return backendCall('users_create_user', { input });
  await delay();
  assertUnique(input.username);
  if (!input.password) throw new ApiError('كلمة المرور مطلوبة للمستخدم الجديد');
  const { password, ...fields } = input;
  const user: User = { id: uid('usr'), ...fields, username: fields.username.trim() };
  mutate(() => {
    db.users.push(user);
    db.credentials[user.username] = password;
  });
  logActivity('user', `إضافة المستخدم ${user.name}`, session.userId, new Date().toISOString(), { name: 'user-editor', params: { id: user.id } });
  return clone(user);
});

export const updateUser = wrap('users.updateUser', async function updateUser(id: string, input: UserInput): Promise<User> {
  if (usesRust('users')) return backendCall('users_update_user', { id, input });
  await delay();
  const user = db.users.find((u) => u.id === id);
  if (!user) throw new ApiError('المستخدم غير موجود', 'NOT_FOUND');
  assertUnique(input.username, id);
  if (id === session.userId && (!input.active || input.role !== user.role)) {
    throw new ApiError('لا يمكنك إيقاف حسابك أو تغيير صلاحيتك بنفسك');
  }
  const { password, ...fields } = input;
  const oldUsername = user.username;
  mutate(() => {
    Object.assign(user, fields, { username: fields.username.trim(), priceListId: fields.priceListId || undefined });
    if (oldUsername !== user.username) {
      db.credentials[user.username] = db.credentials[oldUsername];
      delete db.credentials[oldUsername];
    }
    if (password) db.credentials[user.username] = password;
  });
  logActivity('user', `تعديل بيانات المستخدم ${user.name}`, session.userId, new Date().toISOString(), { name: 'user-editor', params: { id: user.id } });
  return clone(user);
});
