import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { diagnosticsService } from "@@diagnostics/diagnostics.service";

/**
 * Admin surface for C4 (remote diagnostics pull). `getDownloadUrl` already writes the
 * `admin_activity` row itself (non-negotiable per docs/06-security.md), so this controller stays a
 * thin pass-through.
 */
class AdminDiagnosticsController {
    create = AsyncHandler(async (req: Request, res: Response) => {
        const { deviceId, orgId } = req.body as { deviceId?: string; orgId?: string };
        const created = deviceId
            ? await diagnosticsService.createRequest({ deviceIdOrOrgId: deviceId, kind: "device", requestedByAdminId: req.admin!.id })
            : await diagnosticsService.createRequest({ deviceIdOrOrgId: orgId!, kind: "org", requestedByAdminId: req.admin!.id });

        ApiResponse.success({ res, statusCode: 201, message: "تم إنشاء طلب التشخيص", data: created });
    }, "AdminDiagnosticsController.create");

    list = AsyncHandler(async (req: Request, res: Response) => {
        const status = req.query.status as "pending" | "uploaded" | "declined" | "expired" | undefined;
        const result = await diagnosticsService.listForAdmin({
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
    }, "AdminDiagnosticsController.list");

    download = AsyncHandler(async (req: Request, res: Response) => {
        const url = await diagnosticsService.getDownloadUrl(req.params.id, req.admin!.id, req.ip);
        ApiResponse.success({ res, data: { url } });
    }, "AdminDiagnosticsController.download");
}

export const adminDiagnosticsController = new AdminDiagnosticsController();
export default adminDiagnosticsController;
