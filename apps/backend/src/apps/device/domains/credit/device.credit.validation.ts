import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";

const consumeBodySchema = z.object({
    feature: z.string().min(1, "الميزة مطلوبة"),
});

/**
 * `feature` is validated to be a non-empty string here; whether it's a real, creditable ("action")
 * catalog key is checked by creditService.consume itself (400 `not_creditable`), which is the one
 * place that must enforce it since the catalog is the source of truth — this schema doesn't
 * duplicate that check with a hardcoded enum that could drift from `ACTION_FEATURE_KEYS`.
 */
export const deviceCreditValidation = {
    consume: validate({ body: consumeBodySchema }),
};

export type ConsumeCreditBody = z.infer<typeof consumeBodySchema>;

export default deviceCreditValidation;
