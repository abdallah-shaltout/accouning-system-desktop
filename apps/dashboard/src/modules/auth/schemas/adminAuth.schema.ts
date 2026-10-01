import { z } from "zod";

/** Mirrors `apps/backend/src/apps/admin/services/auth/auth.validation.ts` (loginSchema). */
export const adminLoginSchema = z.object({
  email: z.string().email("البريد الإلكتروني غير صالح"),
  password: z.string().min(1, "كلمة المرور مطلوبة"),
});
export type AdminLoginInput = z.infer<typeof adminLoginSchema>;

/** Mirrors `apps/backend/src/apps/admin/services/auth/auth.validation.ts` (totpSchema). */
export const adminTotpSchema = z.object({
  challengeId: z.string().min(1, "معرّف التحقق مطلوب"),
  code: z
    .string()
    .trim()
    .regex(/^\d{6}$/, "رمز التحقق يجب أن يتكون من 6 أرقام"),
});
export type AdminTotpInput = z.infer<typeof adminTotpSchema>;

/** `POST /admin/auth/login` response data. */
export const adminLoginResponseSchema = z.object({
  totpRequired: z.boolean(),
  challengeId: z.string(),
});
export type AdminLoginResponse = z.infer<typeof adminLoginResponseSchema>;

/** `POST /admin/auth/totp` response data. */
export const adminSessionSchema = z.object({
  token: z.string(),
  admin: z.object({
    id: z.string(),
    email: z.string(),
    name: z.string(),
    role: z.string(),
  }),
});
export type AdminSession = z.infer<typeof adminSessionSchema>;
