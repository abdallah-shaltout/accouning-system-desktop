import { Router } from "express";
import { PortalRequiredAuth } from "@@shared/middleware/auth/portal.protacted";
import { portalActivationController } from "./portal.activation.controller";
import { portalActivationValidation } from "./portal.activation.validation";

const router = Router();

router.use(PortalRequiredAuth);

router.post("/approve", portalActivationValidation.approve, portalActivationController.approve);

export default router;
