import { http } from "@/shared/api/http";
import type { ApiResponse } from "@/shared/types/api";
import { useAdminSession, usePortalSession } from "@/shared/auth/useSession";
import {
  adminLoginSchema,
  adminTotpSchema,
  adminLoginResponseSchema,
  adminSessionSchema,
  type AdminLoginInput,
  type AdminTotpInput,
} from "../schemas/adminAuth.schema";
import { portalLoginSchema, portalSessionSchema, type PortalLoginInput } from "../schemas/portalAuth.schema";

/**
 * The ONLY place that calls `shared/api/http` for the admin/portal login flows (module rule:
 * "services: the only place that calls shared/api/http"). Every response is parsed through its Zod
 * schema before the session store or a page ever sees it, so a shape drift from the backend fails
 * loudly here instead of silently in a template.
 */
export const authService = {
  /** `POST /admin/auth/login` — email + password, first step of the admin flow. */
  async adminLogin(input: AdminLoginInput) {
    const parsed = adminLoginSchema.parse(input);
    const { data } = await http.post<ApiResponse<unknown>>(
      "/admin/auth/login",
      parsed,
      { realm: "admin" },
    );
    return adminLoginResponseSchema.parse(data.data);
  },

  /** `POST /admin/auth/totp` — challengeId + 6-digit code, completes the admin flow. */
  async adminTotp(input: AdminTotpInput) {
    const parsed = adminTotpSchema.parse(input);
    const { data } = await http.post<ApiResponse<unknown>>(
      "/admin/auth/totp",
      parsed,
      { realm: "admin" },
    );
    const session = adminSessionSchema.parse(data.data);
    useAdminSession().setSession(session.token, session.admin);
    return session;
  },

  /** `POST /portal/auth/login` — phone + password. */
  async portalLogin(input: PortalLoginInput) {
    const parsed = portalLoginSchema.parse(input);
    const { data } = await http.post<ApiResponse<unknown>>(
      "/portal/auth/login",
      parsed,
      { realm: "portal" },
    );
    const session = portalSessionSchema.parse(data.data);
    usePortalSession().setSession(session.token, session.user);
    return session;
  },
};

export default authService;
