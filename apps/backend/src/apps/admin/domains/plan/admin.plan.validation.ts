import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { uuidParam } from "@@shared/middleware/validator/commonValidator";
import { entitlementsSchema } from "@@plan/schema/entitlements.schema";

const priceField = z.number().int().positive();

const versionIdParam = z.object({
    id: z.string().uuid("معرف الإصدار غير صالح"),
});

const createDraftVersionBody = z.object({
    priceMonthly: priceField,
    priceYearly: priceField,
    entitlements: entitlementsSchema,
});

const editDraftBody = z
    .object({
        priceMonthly: priceField.optional(),
        priceYearly: priceField.optional(),
        entitlements: entitlementsSchema.optional(),
    })
    .refine((data) => data.priceMonthly !== undefined || data.priceYearly !== undefined || data.entitlements !== undefined, {
        message: "لا يوجد ما يتم تعديله",
    });

export const adminPlanValidation = {
    planIdParam: validate({ params: uuidParam }),
    versionIdParam: validate({ params: versionIdParam }),
    createDraftVersion: [validate({ params: uuidParam, body: createDraftVersionBody })],
    editDraft: [validate({ params: versionIdParam, body: editDraftBody })],
    versionAction: [validate({ params: versionIdParam })],
};

export default adminPlanValidation;
