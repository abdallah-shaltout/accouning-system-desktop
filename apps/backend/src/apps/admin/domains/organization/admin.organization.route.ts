import { Router } from "express";
import { AdminRequiredAuth, AdminAllowTo } from "@@shared/middleware/auth/admin.protacted";
import { adminOrganizationController } from "./admin.organization.controller";
import { adminOrganizationValidation } from "./admin.organization.validation";

const router = Router();

router.use(AdminRequiredAuth);

router.get("/", adminOrganizationValidation.list, adminOrganizationController.readDocument);
router.get("/:id", adminOrganizationValidation.detail, adminOrganizationController.detail);
router.patch(
    "/:id",
    AdminAllowTo("owner", "support"),
    adminOrganizationValidation.update,
    adminOrganizationController.updateDocument,
);

export default router;
