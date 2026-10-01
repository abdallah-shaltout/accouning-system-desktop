import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";

const checkoutBody = z.object({
    planVersionId: z.string().uuid("معرف إصدار الباقة غير صالح"),
    interval: z.enum(["month", "year"]),
});

export const portalSubscriptionValidation = {
    checkout: [validate({ body: checkoutBody })],
};

export default portalSubscriptionValidation;
