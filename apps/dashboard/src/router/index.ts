import { createRouter, createWebHistory } from "vue-router";
import { installAuthGuards } from "@/shared/auth/guards";
import { authRoutes } from "@/modules/auth/routes";
import { coreRoutes } from "@/modules/core/routes";

/**
 * History mode (this is a real website, not the Tauri hash-routed desktop app — docs/03-architecture.md
 * "Routing"). Every module contributes its own `routes.ts`; collected here, never declared inline.
 */
export const router = createRouter({
  history: createWebHistory(),
  routes: [...authRoutes, ...coreRoutes],
});

router.beforeEach((to) => {
  if (typeof to.meta.title === "string") {
    document.title = `${to.meta.title} — Equal`;
  }
});

installAuthGuards(router);

export default router;
