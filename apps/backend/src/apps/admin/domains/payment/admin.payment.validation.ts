import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { uuidParam } from "@@shared/middleware/validator/commonValidator";

const listQuery = z.object({
    status: z.enum(["pending", "approved", "rejected"]).optional(),
});

const rejectBody = z.object({
    reason: z.string().min(1, "سبب الرفض مطلوب").max(500),
});

export const adminPaymentValidation = {
    list: [validate({ query: listQuery })],
    idParam: [validate({ params: uuidParam })],
    reject: [validate({ params: uuidParam, body: rejectBody })],
};

export default adminPaymentValidation;
