import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { textField } from "@@shared/middleware/validator/commonValidator";

const updateOrganizationSchema = z.object({
    name: textField({ min: 2, max: 150, fieldName: "اسم المنشأة" }).optional(),
    governorate: textField({ min: 2, max: 100, fieldName: "المحافظة" }).optional(),
    area: textField({ min: 2, max: 100, fieldName: "المنطقة" }).optional(),
});

export const portalOrganizationValidation = {
    update: validate({ body: updateOrganizationSchema }),
};

export type UpdateOrganizationBody = z.infer<typeof updateOrganizationSchema>;

export default portalOrganizationValidation;
