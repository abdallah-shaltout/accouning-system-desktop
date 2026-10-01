import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { adminActivityService } from "@@adminActivity/adminActivity.service";
import { releaseService } from "@@release/release.service";
import type { CreateReleaseBody, PublishReleaseBody } from "./admin.release.validation";

/**
 * `/api/admin/releases`: create (multipart upload to R2), publish/raise-rollout, pause, list,
 * adoption. Every write is state-transition-shaped rather than generic CRUD, so this logs to
 * `adminActivityService` directly instead of extending an `AdminBaseController`.
 */
class AdminReleaseController {
    private readonly targetType = "release";

    list = AsyncHandler(async (_req: Request, res: Response) => {
        const rows = await releaseService.listAll();
        ApiResponse.success({ res, data: rows });
    }, "AdminReleaseController.list");

    adoption = AsyncHandler(async (_req: Request, res: Response) => {
        const rows = await releaseService.adoptionByVersion();
        ApiResponse.success({ res, data: rows });
    }, "AdminReleaseController.adoption");

    /**
     * Accepts the NSIS installer as `installer` (multipart file field) and the Tauri `.sig` file's
     * text contents as `signature` (a plain text field — the signature is small enough that a
     * separate file upload would add nothing but a second multer field to manage).
     */
    create = AsyncHandler(async (req: Request, res: Response) => {
        const file = (req as any).file as { buffer: Buffer; originalname: string } | undefined;
        if (!file) {
            throw new ApiError({ statusCode: 422, message: "ملف التثبيت مطلوب", code: "validation_failed" });
        }

        const body = req.body as CreateReleaseBody;

        const created = await releaseService.createRelease({
            version: body.version,
            channel: body.channel,
            notes: body.notes,
            fileBuffer: file.buffer,
            fileName: file.originalname,
            signature: body.signature,
            isMandatory: body.isMandatory,
        });

        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "create",
            targetType: this.targetType,
            targetId: created.id,
            after: created,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });

        ApiResponse.success({ res, statusCode: 201, message: "تم إنشاء الإصدار", data: created });
    }, "AdminReleaseController.create");

    publish = AsyncHandler(async (req: Request, res: Response) => {
        const { rolloutPercent } = req.body as PublishReleaseBody;
        const before = await releaseService.readDocumentById(req.params.id);
        const updated = await releaseService.publish(req.params.id, rolloutPercent);

        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "publish",
            targetType: this.targetType,
            targetId: req.params.id,
            before,
            after: updated,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });

        ApiResponse.success({ res, message: "تم تحديث نسبة الطرح", data: updated });
    }, "AdminReleaseController.publish");

    pause = AsyncHandler(async (req: Request, res: Response) => {
        const before = await releaseService.readDocumentById(req.params.id);
        const updated = await releaseService.pause(req.params.id);

        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "pause",
            targetType: this.targetType,
            targetId: req.params.id,
            before,
            after: updated,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });

        ApiResponse.success({ res, message: "تم إيقاف الطرح", data: updated });
    }, "AdminReleaseController.pause");
}

export const adminReleaseController = new AdminReleaseController();
export default adminReleaseController;
