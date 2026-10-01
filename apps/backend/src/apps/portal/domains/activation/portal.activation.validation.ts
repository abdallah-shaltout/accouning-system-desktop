import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";
import { textField } from "@@shared/middleware/validator/commonValidator";

const approveSchema = z.object({
    challenge: textField({ min: 16, max: 512, fieldName: "challenge" }),
    state: textField({ min: 1, max: 200, fieldName: "state" }),
    terminalId: textField({ min: 1, max: 200, fieldName: "معرف الجهاز" }),
    deviceName: textField({ min: 1, max: 100, fieldName: "اسم الجهاز" }).optional(),
});

export const portalActivationValidation = {
    approve: validate({ body: approveSchema }),
};

export type ApproveActivationBody = z.infer<typeof approveSchema>;

export default portalActivationValidation;
