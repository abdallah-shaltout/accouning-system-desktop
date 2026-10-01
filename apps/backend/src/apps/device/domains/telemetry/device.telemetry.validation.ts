import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";

const MAX_BATCH_SIZE = 200;

const errorItemSchema = z.object({
    fingerprint: z.string().min(1, "بصمة الخطأ مطلوبة"),
    code: z.string().min(1, "رمز الخطأ مطلوب"),
    source: z.string().min(1, "مصدر الخطأ مطلوب"),
    message: z.string().min(1, "نص الخطأ مطلوب"),
    appVersion: z.string().min(1, "إصدار التطبيق مطلوب"),
    count: z.number().int().positive(),
    firstSeen: z.string().datetime(),
    lastSeen: z.string().datetime(),
});

const errorsBodySchema = z.array(errorItemSchema).max(MAX_BATCH_SIZE, `الحد الأقصى ${MAX_BATCH_SIZE} عنصر لكل دفعة`);

export const deviceTelemetryValidation = {
    reportErrors: validate({ body: errorsBodySchema }),
};

export type ErrorItemBody = z.infer<typeof errorItemSchema>;
export type ErrorsBody = z.infer<typeof errorsBodySchema>;

export default deviceTelemetryValidation;
