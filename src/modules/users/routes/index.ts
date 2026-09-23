import type { RouteRecordRaw } from 'vue-router';

const section = 'الإدارة';

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'login',
    component: () => import('../pages/LoginPage.vue'),
    meta: { public: true, layout: 'blank', title: 'تسجيل الدخول' },
  },
  {
    path: '/users',
    name: 'users',
    component: () => import('../pages/UserListPage.vue'),
    meta: { title: 'المستخدمين', section, area: 'users' },
  },
  {
    path: '/users/:id',
    name: 'user-editor',
    component: () => import('../pages/UserEditorPage.vue'),
    meta: { title: 'بيانات المستخدم', section, area: 'users', access: 'write' },
  },
];

export default routes;
