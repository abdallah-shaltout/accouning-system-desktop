import type { RouteRecordRaw } from "vue-router";
import AdminLayout from "@/shared/layouts/AdminLayout.vue";
import PortalLayout from "@/shared/layouts/PortalLayout.vue";

/**
 * D1 placeholder routes proving AdminLayout/PortalLayout + the auth guard work end-to-end
 * (docs/04-screens.md `admin-home`, `portal-home`). D2/D3 replace these pages with the real
 * `analytics`/`subscriptions` module pages without changing the route names.
 */
export const coreRoutes: RouteRecordRaw[] = [
  {
    path: "/admin",
    component: AdminLayout,
    children: [
      {
        path: "",
        name: "admin-home",
        component: () => import("./pages/AdminHomePlaceholderPage.vue"),
        meta: { title: "لوحة التحكم", area: "admin" },
      },
    ],
  },
  {
    path: "/portal",
    component: PortalLayout,
    children: [
      {
        path: "",
        name: "portal-home",
        component: () => import("./pages/PortalHomePlaceholderPage.vue"),
        meta: { title: "حسابك", area: "portal" },
      },
    ],
  },
  {
    path: "/:pathMatch(.*)*",
    name: "not-found",
    component: () => import("./pages/NotFoundPage.vue"),
    meta: { title: "غير موجود", area: "public" },
  },
];

export default coreRoutes;
