import { Router } from "express";
import { PortalRequiredAuth, PortalAllowTo } from "@@shared/middleware/auth/portal.protacted";
import { portalSubscriptionController } from "./portal.subscription.controller";
import { portalSubscriptionValidation } from "./portal.subscription.validation";

const router = Router();

router.use(PortalRequiredAuth);

router.get("/", portalSubscriptionController.getCurrent);
router.post("/checkout", PortalAllowTo("owner"), portalSubscriptionValidation.checkout, portalSubscriptionController.checkout);
router.post("/cancel", PortalAllowTo("owner"), portalSubscriptionController.cancel);
router.post("/resume", PortalAllowTo("owner"), portalSubscriptionController.resume);

export default router;
