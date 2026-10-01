import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";

const submitFeedbackBody = z.object({
    message: z.string().min(1, "الرسالة مطلوبة").max(4000),
});

export type SubmitFeedbackBody = z.infer<typeof submitFeedbackBody>;

export const deviceFeedbackValidation = {
    submit: [validate({ body: submitFeedbackBody })],
};

export default deviceFeedbackValidation;
