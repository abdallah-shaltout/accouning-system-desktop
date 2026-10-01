import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { uuidParam } from "@@shared/middleware/validator/commonValidator";

const listQuery = z.object({
    status: z.enum(["pending", "uploaded", "declined", "expired"]).optional(),
    page: z.coerce.number().int().positive().optional(),
    limit: z.coerce.number().int().positive().max(100).optional(),
});

const createRequestBody = z
    .object({
        deviceId: z.string().uuid("معرف الجهاز غير صالح").optional(),
        orgId: z.string().uuid("معرف المنشأة غير صالح").optional(),
    })
    .refine((data) => Boolean(data.deviceId) !== Boolean(data.orgId), {
        message: "يجب تحديد deviceId أو orgId وليس كلاهما",
    });

export const adminDiagnosticsValidation = {
    list: [validate({ query: listQuery })],
    idParam: [validate({ params: uuidParam })],
    create: [validate({ body: createRequestBody })],
};

export default adminDiagnosticsValidation;
