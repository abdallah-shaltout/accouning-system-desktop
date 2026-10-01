import { Router } from "express";
import multer from "multer";
import { DeviceRequiredAuth } from "@@shared/middleware/auth/device.protected";
import { uuidParam } from "@@shared/middleware/validator/commonValidator";
import { validate } from "@@shared/middleware/validator/validation.core";
import { deviceDiagnosticsController } from "./device.diagnostics.controller";

// 20 MB cap per C4's task list; the service double-checks sizeBytes defensively since multer's
// limit error handling can be awkward.
const upload = multer({ limits: { fileSize: 20 * 1024 * 1024 } });

const router = Router();

router.use(DeviceRequiredAuth);

router.put("/:id", upload.single("bundle"), validate({ params: uuidParam }), deviceDiagnosticsController.fulfillOrDecline);

export default router;
