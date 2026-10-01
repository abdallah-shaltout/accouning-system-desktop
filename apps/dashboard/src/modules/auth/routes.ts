import type { RouteRecordRaw } from "vue-router";
import AuthLayout from "@/shared/layouts/AuthLayout.vue";

/**
 * Auth module routes (docs/04-screens.md "Public and auth"). D1 only wires `admin-login` and
 * `portal-login` (enough to prove the scaffold end-to-end) — signup/forgot land in D3.
 */
export const authRoutes: RouteRecordRaw[] = [
  {
    path: "/admin/login",
    component: AuthLayout,
    children: [
      {
        path: "",
        name: "admin-login",
        component: () => import("./pages/admin/AdminLoginPage.vue"),
        meta: { title: "تسجيل الدخول", area: "public" },
      },
    ],
  },
  {
    path: "/login",
    component: AuthLayout,
    children: [
      {
        path: "",
        name: "portal-login",
        component: () => import("./pages/portal/PortalLoginPage.vue"),
        meta: { title: "تسجيل الدخول", area: "public" },
      },
    ],
  },
];

export default authRoutes;
