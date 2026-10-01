import { defineStore } from "pinia";

export type Realm = "admin" | "portal";

/** Minimal user shape common to both realms — modules can widen this via their own types. */
export interface SessionUser {
  id: string;
  [key: string]: unknown;
}

export interface SessionState {
  token: string | null;
  user: SessionUser | null;
}

/**
 * One Pinia store factory shared by both realms (CLAUDE.md "Auth": "the access token lives in memory
 * only, never localStorage"). `admin` and `portal` get fully independent stores/state — no
 * `persist` plugin, nothing written to browser storage, ever. `login`/`logout`/`silentRefresh` call
 * the realm's auth service (passed in by the caller, since `shared/auth` must not depend on a
 * module's `services/`) so this file stays a pure state container plus the auth *mechanics* the
 * router guards and the http interceptor need.
 */
function createSessionStore(realm: Realm) {
  return defineStore(`session-${realm}`, {
    state: (): SessionState => ({
      token: null,
      user: null,
    }),
    getters: {
      isAuthenticated: (state): boolean => state.token !== null,
    },
    actions: {
      /** Called once login/TOTP (admin) or login (portal) succeeds with a fresh access token + user. */
      setSession(token: string, user: SessionUser): void {
        this.token = token;
        this.user = user;
      },
      /** Called by the http.ts refresh interceptor once a silent refresh returns a new access token. */
      setToken(token: string): void {
        this.token = token;
      },
      /** Clears in-memory state only — the refresh cookie is cleared server-side by /auth/logout. */
      clear(): void {
        this.token = null;
        this.user = null;
      },
    },
  });
}

export const useAdminSession = createSessionStore("admin");
export const usePortalSession = createSessionStore("portal");

/** Resolve the right session store for a realm without a caller needing its own if/else. */
export function useSession(realm: Realm) {
  return realm === "admin" ? useAdminSession() : usePortalSession();
}
