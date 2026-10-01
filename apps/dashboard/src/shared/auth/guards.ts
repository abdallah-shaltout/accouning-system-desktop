import type { Router } from "vue-router";
import { onSessionExpired } from "@/shared/api/http";
import { useAdminSession, usePortalSession, type Realm } from "./useSession";

declare module "vue-router" {
  interface RouteMeta {
    /** Which realm a route belongs to. Unguarded when omitted or "public". */
    area?: Realm | "public";
    /** Admin-only: roles allowed to view this route. Omit to allow any authenticated admin. */
    roles?: string[];
  }
}

function loginRouteFor(realm: Realm) {
  return realm === "admin" ? { name: "admin-login" } : { name: "portal-login" };
}

/**
 * Installs the area/role navigation guard plus the session-expired redirect (CLAUDE.md "Auth":
 * "Guards read meta.area and meta.roles. The UI hides what a role can't do, but the backend is the
 * authority."). Call once from `router/index.ts` after the router is created.
 */
export function installAuthGuards(router: Router): void {
  router.beforeEach((to) => {
    const area = to.meta.area;
    if (!area || area === "public") return true;

    const session = area === "admin" ? useAdminSession() : usePortalSession();
    if (!session.isAuthenticated) {
      return { ...loginRouteFor(area), query: { redirect: to.fullPath } /* route-ok: URL round-trip back after login */ };
    }

    if (area === "admin" && to.meta.roles?.length) {
      const role = session.user?.role as string | undefined;
      if (!role || !to.meta.roles.includes(role)) {
        return { name: "admin-home" };
      }
    }

    return true;
  });

  // A refresh failure anywhere (http.ts) clears the realm's session; send the user back to that
  // realm's login from wherever they currently are, once, via the router rather than a full reload.
  onSessionExpired((realm) => {
    const current = router.currentRoute.value;
    if (current.meta.area !== realm) return;
    void router.push({ ...loginRouteFor(realm), query: { redirect: current.fullPath } });
  });
}
