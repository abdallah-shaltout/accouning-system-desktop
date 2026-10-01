import { Router } from "express";
import { AdminRequiredAuth, AdminAllowTo } from "@@shared/middleware/auth/admin.protacted";
import { adminFeedbackController } from "./admin.feedback.controller";
import { adminFeedbackValidation } from "./admin.feedback.validation";

const router = Router();

router.use(AdminRequiredAuth);

router.get("/", adminFeedbackValidation.list, adminFeedbackController.list);
router.patch("/:id", AdminAllowTo("owner", "support"), adminFeedbackValidation.updateStatus, adminFeedbackController.updateStatus);

export default router;
