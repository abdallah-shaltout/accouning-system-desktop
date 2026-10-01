import type { RouteRecordRaw } from 'vue-router';

const section = 'الإدارة';

const routes: RouteRecordRaw[] = [
  {
    path: '/welcome',
    name: 'welcome',
    component: () => import('../pages/WelcomePage.vue'),
    meta: { public: true, layout: 'blank', title: 'مرحباً بك' },
  },
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
  // Self-service — open to every signed-in user regardless of their `users` area permission
  // (no `area` meta), same pattern as settings-appearance. Linked only from NavUser.vue.
  {
    path: '/profile',
    name: 'profile',
    component: () => import('../pages/ProfilePage.vue'),
    meta: { title: 'ملفي الشخصي' },
  },
  {
    path: '/my-activity',
    name: 'my-activity',
    component: () => import('../pages/MyActivityPage.vue'),
    meta: { title: 'نشاطي' },
  },
];

export default routes;
