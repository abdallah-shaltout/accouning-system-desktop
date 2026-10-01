import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { uuidParam } from "@@shared/middleware/validator/commonValidator";

const listQuery = z.object({
    status: z.enum(["new", "in_progress", "done"]).optional(),
    page: z.coerce.number().int().positive().optional(),
    limit: z.coerce.number().int().positive().max(100).optional(),
});

const updateStatusBody = z.object({
    status: z.enum(["new", "in_progress", "done"]),
});

export const adminFeedbackValidation = {
    list: [validate({ query: listQuery })],
    updateStatus: [validate({ params: uuidParam, body: updateStatusBody })],
};

export default adminFeedbackValidation;
