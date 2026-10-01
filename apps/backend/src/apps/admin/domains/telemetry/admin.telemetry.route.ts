import { Router } from "express";
import { AdminRequiredAuth } from "@@shared/middleware/auth/admin.protacted";
import { adminTelemetryController } from "./admin.telemetry.controller";

// Any admin role may read telemetry per docs/05-api-spec.md's admin table — no AdminAllowTo gate.
const router = Router();

router.use(AdminRequiredAuth);

router.get("/export", adminTelemetryController.exportLedger);
router.get("/errors/:id", adminTelemetryController.groupDetail);
router.get("/errors", adminTelemetryController.listGroups);

export default router;
