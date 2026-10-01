import { Router } from "express";
import { PortalRequiredAuth } from "@@shared/middleware/auth/portal.protacted";
import { portalCreditController } from "./portal.credit.controller";

const router = Router();

router.use(PortalRequiredAuth);

router.get("/", portalCreditController.getUsage);

export default router;
