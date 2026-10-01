import { Router } from "express";
import { DeviceRequiredAuth } from "@@shared/middleware/auth/device.protected";
import { deviceRateLimit } from "@@shared/middleware/rateLimit/authRateLimit";
import { deviceReleaseController } from "./device.release.controller";
import { deviceReleaseValidation } from "./device.release.validation";

const router = Router();

router.use(DeviceRequiredAuth);

router.get("/latest", deviceRateLimit, deviceReleaseValidation.latest, deviceReleaseController.latest);

export default router;
