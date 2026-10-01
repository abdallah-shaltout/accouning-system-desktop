import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { phoneEG, textField, uuidParam, paginationQuery } from "@@shared/middleware/validator/commonValidator";

const passwordField = z
    .string()
    .min(8, "كلمة المرور يجب أن تكون 8 أحرف على الأقل")
    .max(72, "كلمة المرور طويلة جدًا");

const createUserSchema = z.object({
    name: textField({ min: 2, max: 150, fieldName: "الاسم" }),
    phone: phoneEG,
    password: passwordField,
});

export const portalUserValidation = {
    list: validate({ query: paginationQuery }),
    create: validate({ body: createUserSchema }),
    remove: validate({ params: uuidParam }),
};

export type CreatePortalUserBody = z.infer<typeof createUserSchema>;

export default portalUserValidation;
