import { z } from "zod";

/** Egyptian mobile number — mirrors `apps/backend/src/shared/middleware/validator/commonValidator.ts` (phoneEG). */
const phoneEG = z.string().regex(/^01[0125][0-9]{8}$/, "رقم الهاتف غير صالح");

/** Mirrors `apps/backend/src/apps/portal/services/auth/auth.validation.ts` (loginSchema). */
export const portalLoginSchema = z.object({
  phone: phoneEG,
  password: z.string().min(1, "كلمة المرور مطلوبة"),
});
export type PortalLoginInput = z.infer<typeof portalLoginSchema>;

/** `POST /portal/auth/login` response data. */
export const portalSessionSchema = z.object({
  token: z.string(),
  user: z.object({
    id: z.string(),
  }).passthrough(),
});
export type PortalSession = z.infer<typeof portalSessionSchema>;
