import axios, { AxiosError, type AxiosInstance, type InternalAxiosRequestConfig } from "axios";
import type { Realm } from "@/shared/auth/useSession";
import { useAdminSession, usePortalSession } from "@/shared/auth/useSession";
import { ApiError, type ApiErrorShape } from "./errors";

/** Extra per-request config this http layer understands, beyond plain axios options. */
declare module "axios" {
  interface AxiosRequestConfig {
    /** Which realm's session/token this request authenticates as. Defaults to inferring from the URL. */
    realm?: Realm;
  }
  interface InternalAxiosRequestConfig {
    realm?: Realm;
    /** Set once a request has already been retried after a silent refresh, to avoid retry loops. */
    _retried?: boolean;
  }
}

/**
 * Fired the moment a realm's session becomes unauthenticated after a failed refresh. `shared/auth/
 * guards.ts` listens to this to redirect to that realm's login — kept out of this file so the
 * interceptor stays focused on token mechanics, not navigation (D1 instructions).
 */
type SessionExpiredListener = (realm: Realm) => void;
const sessionExpiredListeners = new Set<SessionExpiredListener>();
export function onSessionExpired(listener: SessionExpiredListener): () => void {
  sessionExpiredListeners.add(listener);
  return () => sessionExpiredListeners.delete(listener);
}
function notifySessionExpired(realm: Realm): void {
  for (const listener of sessionExpiredListeners) listener(realm);
}

function sessionStoreFor(realm: Realm) {
  return realm === "admin" ? useAdminSession() : usePortalSession();
}

/** `/api/admin/...` -> "admin", `/api/portal/...` -> "portal". Falls back to "portal" (the wider surface). */
function inferRealm(url: string | undefined): Realm {
  if (url?.includes("/admin/")) return "admin";
  return "portal";
}

function isAuthEndpoint(url: string | undefined): boolean {
  if (!url) return false;
  return /\/(admin|portal)\/auth\//.test(url);
}

export const http: AxiosInstance = axios.create({
  baseURL: import.meta.env.VITE_API_URL,
  withCredentials: true, // sends the httpOnly refresh cookie
});

http.interceptors.request.use((config: InternalAxiosRequestConfig) => {
  const realm = config.realm ?? inferRealm(config.url);
  const session = sessionStoreFor(realm);
  if (session.token && !config.headers.Authorization) {
    config.headers.Authorization = `Bearer ${session.token}`;
  }
  return config;
});

/** One shared in-flight refresh promise per realm — dedupes concurrent 401s (D1 instructions). */
const refreshPromises: Partial<Record<Realm, Promise<string>>> = {};

async function refreshRealmToken(realm: Realm): Promise<string> {
  const existing = refreshPromises[realm];
  if (existing) return existing;

  const promise = (async () => {
    try {
      const response = await axios.post<{ data: { token: string } }>(
        `${import.meta.env.VITE_API_URL}/${realm}/auth/refresh`,
        undefined,
        { withCredentials: true },
      );
      const token = response.data.data.token;
      sessionStoreFor(realm).setToken(token);
      return token;
    } catch (err) {
      sessionStoreFor(realm).clear();
      notifySessionExpired(realm);
      throw err;
    } finally {
      delete refreshPromises[realm];
    }
  })();

  refreshPromises[realm] = promise;
  return promise;
}

http.interceptors.response.use(
  (response) => response,
  async (error: AxiosError<ApiErrorShape>) => {
    const config = error.config as InternalAxiosRequestConfig | undefined;

    if (
      error.response?.status === 401 &&
      config &&
      !config._retried &&
      !isAuthEndpoint(config.url)
    ) {
      const realm = config.realm ?? inferRealm(config.url);
      try {
        const token = await refreshRealmToken(realm);
        config._retried = true;
        config.headers.Authorization = `Bearer ${token}`;
        return http(config);
      } catch {
        // Falls through to the ApiError rejection below; the session is already cleared.
      }
    }

    if (error.response?.data) {
      throw new ApiError(error.response.data);
    }
    throw new ApiError({
      status: "error",
      success: false,
      statusCode: error.response?.status ?? 0,
      message: error.message || "تعذّر الاتصال بالخادم",
    });
  },
);

export default http;
