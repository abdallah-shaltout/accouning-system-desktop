import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { textField } from "@@shared/middleware/validator/commonValidator";

const registerSchema = z.object({
    terminalId: textField({ min: 1, max: 200, fieldName: "معرف الجهاز" }),
    appVersion: textField({ min: 1, max: 50, fieldName: "إصدار التطبيق" }).optional(),
    os: textField({ min: 1, max: 100, fieldName: "نظام التشغيل" }).optional(),
    role: z.enum(["main", "terminal"]),
});

const activateSchema = z
    .object({
        code: z.string().uuid("رمز التفعيل غير صالح").optional(),
        userCode: textField({ min: 6, max: 6, fieldName: "رمز التفعيل" }).optional(),
        verifier: textField({ min: 16, max: 512, fieldName: "verifier" }),
    })
    .refine((data) => Boolean(data.code) !== Boolean(data.userCode), {
        message: "أدخل code أو userCode، وليس كلاهما",
    });

const heartbeatSchema = z.object({
    appVersion: textField({ min: 1, max: 50, fieldName: "إصدار التطبيق" }).optional(),
    os: textField({ min: 1, max: 100, fieldName: "نظام التشغيل" }).optional(),
    telemetryEnabled: z.boolean().optional(),
    diagnosticsAllowed: z.boolean().optional(),
});

export const deviceValidation = {
    register: validate({ body: registerSchema }),
    activate: validate({ body: activateSchema }),
    heartbeat: validate({ body: heartbeatSchema }),
};

export type RegisterDeviceBody = z.infer<typeof registerSchema>;
export type ActivateDeviceBody = z.infer<typeof activateSchema>;
export type HeartbeatBody = z.infer<typeof heartbeatSchema>;

export default deviceValidation;
