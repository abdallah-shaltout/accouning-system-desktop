import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { phoneEG, textField } from "@@shared/middleware/validator/commonValidator";

const passwordField = z
    .string()
    .min(8, "كلمة المرور يجب أن تكون 8 أحرف على الأقل")
    .max(72, "كلمة المرور طويلة جدًا");

const otpCodeField = z
    .string()
    .regex(/^[0-9]{6}$/, "رمز التحقق غير صالح");

const signupSchema = z.object({
    phone: phoneEG,
    password: passwordField,
    orgName: textField({ min: 2, max: 150, fieldName: "اسم المنشأة" }),
});

const verifyOtpSchema = z.object({
    phone: phoneEG,
    code: otpCodeField,
});

const loginSchema = z.object({
    phone: phoneEG,
    password: z.string().min(1, "كلمة المرور مطلوبة"),
});

const forgotSchema = z.object({
    phone: phoneEG,
});

const resetSchema = z.object({
    phone: phoneEG,
    code: otpCodeField,
    newPassword: passwordField,
});

export const authValidation = {
    signup: validate({ body: signupSchema }),
    verifyOtp: validate({ body: verifyOtpSchema }),
    login: validate({ body: loginSchema }),
    forgot: validate({ body: forgotSchema }),
    reset: validate({ body: resetSchema }),
};

export type SignupBody = z.infer<typeof signupSchema>;
export type VerifyOtpBody = z.infer<typeof verifyOtpSchema>;
export type LoginBody = z.infer<typeof loginSchema>;
export type ForgotBody = z.infer<typeof forgotSchema>;
export type ResetBody = z.infer<typeof resetSchema>;

export default authValidation;
