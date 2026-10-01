import { Router } from "express";
import { PortalRequiredAuth } from "@@shared/middleware/auth/portal.protacted";
import { portalPaymentController } from "./portal.payment.controller";

const router = Router();

router.use(PortalRequiredAuth);
router.get("/", portalPaymentController.listInvoices);

export default router;
