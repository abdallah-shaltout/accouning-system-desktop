import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { diagnosticsService } from "@@diagnostics/diagnostics.service";

/**
 * `PUT /device/diagnostics/:id` accepts either a multipart zip (`bundle`) or a JSON
 * `{declined: true}` body. The route runs `multer().single("bundle")` unconditionally — it
 * tolerates a non-multipart request by leaving `req.file` undefined — so this controller only
 * branches on what actually showed up.
 */
class DeviceDiagnosticsController {
    fulfillOrDecline = AsyncHandler(async (req: Request, res: Response) => {
        const declined = req.body?.declined === true || req.body?.declined === "true";
        const file = (req as any).file as { buffer: Buffer; size: number } | undefined;

        if (declined) {
            const updated = await diagnosticsService.decline(req.params.id, req.device!.id);
            ApiResponse.success({ res, message: "تم رفض طلب التشخيص", data: updated });
            return;
        }

        if (file) {
            const updated = await diagnosticsService.fulfillWithUpload(req.params.id, req.device!.id, file.buffer, file.size);
            ApiResponse.success({ res, message: "تم رفع ملف التشخيص", data: updated });
            return;
        }

        throw new ApiError({ statusCode: 422, message: "يجب إرفاق ملف أو تحديد declined", code: "validation_failed" });
    }, "DeviceDiagnosticsController.fulfillOrDecline");
}

export const deviceDiagnosticsController = new DeviceDiagnosticsController();
export default deviceDiagnosticsController;
