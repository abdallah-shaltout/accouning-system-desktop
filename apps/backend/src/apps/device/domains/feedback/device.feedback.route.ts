import { Router } from "express";
import multer from "multer";
import { DeviceRequiredAuth } from "@@shared/middleware/auth/device.protected";
import { deviceFeedbackController } from "./device.feedback.controller";
import { deviceFeedbackValidation } from "./device.feedback.validation";

const upload = multer({ limits: { fileSize: 20 * 1024 * 1024 } });

const router = Router();

router.use(DeviceRequiredAuth);

router.post(
    "/",
    upload.fields([
        { name: "screenshot", maxCount: 1 },
        { name: "bundle", maxCount: 1 },
    ]),
    deviceFeedbackValidation.submit,
    deviceFeedbackController.submit,
);

export default router;
