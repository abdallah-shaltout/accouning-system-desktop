import { Router } from "express";
import { DeviceRequiredAuth } from "@@shared/middleware/auth/device.protected";
import { deviceRateLimit } from "@@shared/middleware/rateLimit/authRateLimit";
import { deviceTelemetryController } from "./device.telemetry.controller";
import { deviceTelemetryValidation } from "./device.telemetry.validation";

const router = Router();

router.use(deviceRateLimit, DeviceRequiredAuth);

router.post("/errors", deviceTelemetryValidation.reportErrors, deviceTelemetryController.reportErrors);

export default router;
