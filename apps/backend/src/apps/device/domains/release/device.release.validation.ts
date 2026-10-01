import { z } from "zod";
import { validate } from "@@shared/middleware/validator/validation.core";

const latestQuery = z.object({
    current: z.string().min(1, "الإصدار الحالي مطلوب"),
    channel: z.enum(["stable", "beta"]).default("stable"),
});

export type LatestReleaseQuery = z.infer<typeof latestQuery>;

export const deviceReleaseValidation = {
    latest: [validate({ query: latestQuery })],
};

export default deviceReleaseValidation;
