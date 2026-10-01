import { Router } from "express";
import { PortalRequiredAuth } from "@@shared/middleware/auth/portal.protacted";
import { portalDeviceController } from "./portal.device.controller";
import { portalDeviceValidation } from "./portal.device.validation";

const router = Router();

router.use(PortalRequiredAuth);

router.get("/", portalDeviceController.list);
router.patch("/:id", portalDeviceValidation.rename, portalDeviceController.rename);
router.post("/:id/deactivate", portalDeviceValidation.deactivate, portalDeviceController.deactivate);

export default router;
