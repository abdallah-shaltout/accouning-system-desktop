import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";

const loginSchema = z.object({
    email: z.string().email("البريد الإلكتروني غير صالح"),
    password: z.string().min(1, "كلمة المرور مطلوبة"),
});

const totpSchema = z.object({
    challengeId: z.string().min(1, "معرّف التحقق مطلوب"),
    code: z
        .string()
        .trim()
        .regex(/^\d{6}$/, "رمز التحقق يجب أن يتكون من 6 أرقام"),
});

export const loginValidation = [validate({ body: loginSchema })];
export const totpValidation = [validate({ body: totpSchema })];

export type LoginInput = z.infer<typeof loginSchema>;
export type TotpInput = z.infer<typeof totpSchema>;
