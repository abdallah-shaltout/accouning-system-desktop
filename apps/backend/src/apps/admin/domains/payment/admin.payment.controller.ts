import type { Request, Response } from "express";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { adminActivityService } from "@@adminActivity/adminActivity.service";
import { paymentService } from "@@payment/payment.service";
import { payment } from "@@payment/payment.schema";

/**
 * Approve/reject are state-transition actions, not plain update-by-id, so this controller calls
 * `adminActivityService.log` directly rather than extending `AdminBaseController`'s generic CRUD.
 */
class AdminPaymentController {
    private readonly targetType = "payment";

    list = AsyncHandler(async (req: Request, res: Response) => {
        const status = req.query.status as "pending" | "approved" | "rejected" | undefined;
        const rows = status
            ? await db.select().from(payment).where(eq(payment.status, status))
            : await paymentService.listPendingForAdmin();
        ApiResponse.success({ res, data: rows });
    }, "AdminPaymentController.list");

    approve = AsyncHandler(async (req: Request, res: Response) => {
        const before = await paymentService.readDocumentById(req.params.id);
        const updated = await paymentService.approvePayment(req.params.id, req.admin!.id);

        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "approve",
            targetType: this.targetType,
            targetId: req.params.id,
            before,
            after: updated,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });

        ApiResponse.success({ res, message: "تم اعتماد عملية الدفع", data: updated });
    }, "AdminPaymentController.approve");

    reject = AsyncHandler(async (req: Request, res: Response) => {
        const { reason } = req.body as { reason: string };
        const before = await paymentService.readDocumentById(req.params.id);
        const updated = await paymentService.rejectPayment(req.params.id, req.admin!.id, reason);

        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "reject",
            targetType: this.targetType,
            targetId: req.params.id,
            before,
            after: updated,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });

        ApiResponse.success({ res, message: "تم رفض عملية الدفع", data: updated });
    }, "AdminPaymentController.reject");
}

export const adminPaymentController = new AdminPaymentController();
export default adminPaymentController;
