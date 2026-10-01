import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { uuidParam } from "@@shared/middleware/validator/commonValidator";

// Multipart body fields arrive as strings — coerce booleans, keep version/channel/notes as text.
const createBody = z.object({
    version: z
        .string()
        .regex(/^\d+\.\d+\.\d+$/, "رقم الإصدار يجب أن يكون بصيغة X.Y.Z"),
    channel: z.enum(["stable", "beta"]),
    notes: z.string().max(5000).optional(),
    signature: z.string().min(1, "توقيع التحديث مطلوب"),
    isMandatory: z
        .union([z.boolean(), z.string()])
        .optional()
        .transform((v) => v === true || v === "true"),
});

const publishBody = z.object({
    rolloutPercent: z.coerce.number().int().min(0).max(100),
});

export type CreateReleaseBody = z.infer<typeof createBody>;
export type PublishReleaseBody = z.infer<typeof publishBody>;

export const adminReleaseValidation = {
    create: [validate({ body: createBody })],
    publish: [validate({ params: uuidParam, body: publishBody })],
    idParam: [validate({ params: uuidParam })],
};

export default adminReleaseValidation;
