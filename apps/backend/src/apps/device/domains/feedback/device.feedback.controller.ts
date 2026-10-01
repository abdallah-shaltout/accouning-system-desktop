import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { feedbackService } from "@@feedback/feedback.service";
import type { SubmitFeedbackBody } from "./device.feedback.validation";

class DeviceFeedbackController {
    submit = AsyncHandler(async (req: Request, res: Response) => {
        const { message } = req.body as SubmitFeedbackBody;
        const files = req.files as Record<string, Express.Multer.File[]> | undefined;
        const screenshot = files?.screenshot?.[0];
        const bundle = files?.bundle?.[0];

        const created = await feedbackService.submit({
            deviceId: req.device!.id,
            orgId: req.device!.orgId,
            message,
            screenshotBuffer: screenshot?.buffer ?? null,
            screenshotMimeType: screenshot?.mimetype,
            bundleBuffer: bundle?.buffer ?? null,
            bundleMimeType: bundle?.mimetype,
        });

        ApiResponse.success({ res, statusCode: 201, message: "تم إرسال الملاحظة، شكرًا لك", data: created });
    }, "DeviceFeedbackController.submit");
}

export const deviceFeedbackController = new DeviceFeedbackController();
export default deviceFeedbackController;
