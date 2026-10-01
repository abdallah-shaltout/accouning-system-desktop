import { Router } from "express";
import { AdminRequiredAuth, AdminAllowTo } from "@@shared/middleware/auth/admin.protacted";
import { adminPlanController } from "./admin.plan.controller";
import { adminPlanValidation } from "./admin.plan.validation";

const router = Router();

// Pricing changes are owner-only (docs/05-api-spec.md "role: owner").
router.get("/", AdminRequiredAuth, AdminAllowTo("owner"), adminPlanController.listPlans);

router.post(
    "/:id/versions",
    AdminRequiredAuth,
    AdminAllowTo("owner"),
    ...adminPlanValidation.createDraftVersion,
    adminPlanController.createDraftVersion,
);

router.put(
    "/versions/:id",
    AdminRequiredAuth,
    AdminAllowTo("owner"),
    ...adminPlanValidation.editDraft,
    adminPlanController.editDraft,
);

router.post(
    "/versions/:id/publish",
    AdminRequiredAuth,
    AdminAllowTo("owner"),
    ...adminPlanValidation.versionAction,
    adminPlanController.publishVersion,
);

router.post(
    "/versions/:id/retire",
    AdminRequiredAuth,
    AdminAllowTo("owner"),
    ...adminPlanValidation.versionAction,
    adminPlanController.retireVersion,
);

export default router;
