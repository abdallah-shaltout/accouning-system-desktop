import { Router } from "express";
import { PortalRequiredAuth, PortalAllowTo } from "@@shared/middleware/auth/portal.protacted";
import { portalOrganizationController } from "./portal.organization.controller";
import { portalOrganizationValidation } from "./portal.organization.validation";

const router = Router();

router.use(PortalRequiredAuth);

router.get("/", portalOrganizationController.getMine);
// Business profile edits are owner-only (mirrors the B1 spec's "owner only" line for /users).
router.put("/", PortalAllowTo("owner"), portalOrganizationValidation.update, portalOrganizationController.updateMine);

export default router;
