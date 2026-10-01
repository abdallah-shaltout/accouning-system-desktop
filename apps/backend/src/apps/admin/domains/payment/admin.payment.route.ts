import { Router } from "express";
import { AdminRequiredAuth, AdminAllowTo } from "@@shared/middleware/auth/admin.protacted";
import { idempotency } from "@@shared/middleware/idempotency";
import { adminPaymentController } from "./admin.payment.controller";
import { adminPaymentValidation } from "./admin.payment.validation";

const router = Router();

router.use(AdminRequiredAuth);
router.use(AdminAllowTo("owner", "finance"));

router.get("/", adminPaymentValidation.list, adminPaymentController.list);
router.post(
    "/:id/approve",
    adminPaymentValidation.idParam,
    idempotency({ scope: "admin.payment.approve", required: true, scopeOwner: (req) => req.admin!.id }),
    adminPaymentController.approve,
);
router.post("/:id/reject", adminPaymentValidation.reject, adminPaymentController.reject);

export default router;
