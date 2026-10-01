import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { uuidParam, paginationQuery, textField } from "@@shared/middleware/validator/commonValidator";

const searchQuery = paginationQuery.extend({
    search: z.string().optional(),
});

const patchOrganizationSchema = z.object({
    notes: textField({ min: 0, max: 5000, fieldName: "الملاحظات" }).optional(),
    status: z.enum(["active", "suspended"]).optional(),
});

export const adminOrganizationValidation = {
    list: validate({ query: searchQuery }),
    detail: validate({ params: uuidParam }),
    update: validate({ params: uuidParam, body: patchOrganizationSchema }),
};

export type PatchOrganizationBody = z.infer<typeof patchOrganizationSchema>;

export default adminOrganizationValidation;
