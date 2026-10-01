import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";

/**
 * `amount` is deliberately absent: the payment amount is always copied from the invoice server-side
 * (CLAUDE.md "never trust client-sent prices"), never accepted from the request body.
 */
const submitPaymentBody = z.object({
    invoiceId: z.string().uuid("معرف الفاتورة غير صالح"),
    method: z.enum(["instapay", "vodafone_cash", "bank", "card", "wallet"]),
    reference: z.string().max(200).optional(),
});

export const portalPaymentValidation = {
    submit: [validate({ body: submitPaymentBody })],
};

export default portalPaymentValidation;
