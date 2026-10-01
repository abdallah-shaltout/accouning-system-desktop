import { Router } from "express";
import { DeviceRequiredAuth } from "@@shared/middleware/auth/device.protected";
import { deviceRegisterRateLimit, activationRateLimit, deviceRateLimit } from "@@shared/middleware/rateLimit/authRateLimit";
import { idempotency } from "@@shared/middleware/idempotency";
import { deviceController } from "./device.controller";
import { deviceValidation } from "./device.validation";

const router = Router();

// Public, IP rate-limited: a device has no credential yet.
router.post("/register", deviceRegisterRateLimit, deviceValidation.register, deviceController.register);

// Device-authenticated + Idempotency-Key: activation must never double-bind/double-issue on retry.
router.post(
    "/activate",
    activationRateLimit,
    DeviceRequiredAuth,
    idempotency({ scope: "device.activate", required: true }),
    deviceValidation.activate,
    deviceController.activate,
);

router.post(
    "/heartbeat",
    deviceRateLimit,
    DeviceRequiredAuth,
    deviceValidation.heartbeat,
    deviceController.heartbeat,
);

export default router;
