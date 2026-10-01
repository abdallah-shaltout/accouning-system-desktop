import { Router } from "express";
import { z } from "zod";
import { AdminRequiredAuth, AdminAllowTo } from "@@shared/middleware/auth/admin.protacted";
import { validate } from "@@shared/middleware/validator/validation.core";
import { adminAnalyticsController } from "./admin.analytics.controller";

const router = Router();

// Metrics are owner-only: the phase C task list doesn't specify per-endpoint roles for analytics,
// so every route in this module uses the strictest admin role consistently.
router.use(AdminRequiredAuth);
router.use(AdminAllowTo("owner"));

const creditsQuery = z.object({
    days: z.coerce.number().int().positive().optional(),
});

router.get("/overview", adminAnalyticsController.overview);
router.get("/credits", validate({ query: creditsQuery }), adminAnalyticsController.credits);
router.get("/versions", adminAnalyticsController.versions);

export default router;
