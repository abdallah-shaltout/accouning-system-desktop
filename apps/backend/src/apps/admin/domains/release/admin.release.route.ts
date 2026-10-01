import { Router } from "express";
import multer from "multer";
import { AdminRequiredAuth, AdminAllowTo } from "@@shared/middleware/auth/admin.protacted";
import { adminReleaseController } from "./admin.release.controller";
import { adminReleaseValidation } from "./admin.release.validation";

// Installers are large (Windows NSIS builds) — a generous limit so a legitimate upload never fails.
const upload = multer({ limits: { fileSize: 200 * 1024 * 1024 } });

const router = Router();

router.use(AdminRequiredAuth);
router.use(AdminAllowTo("owner"));

router.get("/", adminReleaseController.list);
router.get("/adoption", adminReleaseController.adoption);
router.post("/", upload.single("installer"), adminReleaseValidation.create, adminReleaseController.create);
router.post("/:id/publish", adminReleaseValidation.publish, adminReleaseController.publish);
router.post("/:id/pause", adminReleaseValidation.idParam, adminReleaseController.pause);

export default router;
