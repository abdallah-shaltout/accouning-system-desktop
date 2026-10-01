import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { feedbackService } from "@@feedback/feedback.service";

class AdminFeedbackController {
    list = AsyncHandler(async (req: Request, res: Response) => {
        const status = req.query.status as "new" | "in_progress" | "done" | undefined;
        const result = await feedbackService.listForAdmin({
            status,
            page: req.query.page as unknown as number | undefined,
            limit: req.query.limit as unknown as number | undefined,
        });
        ApiResponse.success({
            res,
            data: result.data,
            page: result.page,
            limit: result.limit,
            total: result.total,
            totalPages: result.totalPages,
        });
    }, "AdminFeedbackController.list");

    updateStatus = AsyncHandler(async (req: Request, res: Response) => {
        const { status } = req.body as { status: "new" | "in_progress" | "done" };
        const updated = await feedbackService.updateStatus(req.params.id, status);
        ApiResponse.success({ res, message: "تم تحديث حالة الملاحظة", data: updated });
    }, "AdminFeedbackController.updateStatus");
}

export const adminFeedbackController = new AdminFeedbackController();
export default adminFeedbackController;
