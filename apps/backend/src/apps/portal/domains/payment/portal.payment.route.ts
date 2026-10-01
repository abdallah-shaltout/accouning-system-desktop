import { Router } from "express";
import multer from "multer";
import { PortalRequiredAuth } from "@@shared/middleware/auth/portal.protacted";
import { portalPaymentController } from "./portal.payment.controller";
import { portalPaymentValidation } from "./portal.payment.validation";

const upload = multer({ limits: { fileSize: 10 * 1024 * 1024 } });

const router = Router();

router.use(PortalRequiredAuth);

router.get("/", portalPaymentController.list);
router.post("/", upload.single("receipt"), portalPaymentValidation.submit, portalPaymentController.submit);

export default router;
