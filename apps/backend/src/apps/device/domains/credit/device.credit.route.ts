import { Router } from "express";
import { DeviceRequiredAuth } from "@@shared/middleware/auth/device.protected";
import { deviceRateLimit } from "@@shared/middleware/rateLimit/authRateLimit";
import { deviceCreditController } from "./device.credit.controller";
import { deviceCreditValidation } from "./device.credit.validation";

// Not wrapped with the generic `idempotency()` middleware — see device.credit.controller.ts's
// consume handler for why: creditService.consume already implements ledger-level idempotency
// keyed on the same Idempotency-Key header value.
const router = Router();

router.use(deviceRateLimit, DeviceRequiredAuth);

router.get("/", deviceCreditController.getUsage);
router.post("/consume", deviceCreditValidation.consume, deviceCreditController.consume);

export default router;
