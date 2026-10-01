import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { uuidParam, textField } from "@@shared/middleware/validator/commonValidator";

const renameSchema = z.object({
    name: textField({ min: 1, max: 100, fieldName: "اسم الجهاز" }),
});

export const portalDeviceValidation = {
    idParam: validate({ params: uuidParam }),
    rename: validate({ params: uuidParam, body: renameSchema }),
    deactivate: validate({ params: uuidParam }),
};

export type RenameDeviceBody = z.infer<typeof renameSchema>;

export default portalDeviceValidation;
