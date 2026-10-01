import { Router } from "express";
import { AdminRequiredAuth, AdminAllowTo } from "@@shared/middleware/auth/admin.protacted";
import { adminDiagnosticsController } from "./admin.diagnostics.controller";
import { adminDiagnosticsValidation } from "./admin.diagnostics.validation";

const router = Router();

router.use(AdminRequiredAuth);

router.get("/", adminDiagnosticsValidation.list, adminDiagnosticsController.list);
router.post("/", AdminAllowTo("owner", "support"), adminDiagnosticsValidation.create, adminDiagnosticsController.create);
router.get("/:id/download", AdminAllowTo("owner", "support"), adminDiagnosticsValidation.idParam, adminDiagnosticsController.download);

export default router;
