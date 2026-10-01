import type { Request, Response, NextFunction } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { telemetryService } from "@@telemetry/telemetry.service";

/**
 * Admin error-telemetry surface (plan 23, C3). Reads only — any role may hit these per
 * docs/05-api-spec.md's admin table, so there's no `AdminAllowTo` gate here.
 */
export class AdminTelemetryController {
    listGroups = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const result = await telemetryService.listGroups({
            search: req.query.search as string | undefined,
            versionFilter: req.query.version as string | undefined,
            page: req.query.page ? Number(req.query.page) : undefined,
            limit: req.query.limit ? Number(req.query.limit) : undefined,
        });

        ApiResponse.success({
            res,
            data: result.data,
            page: result.page,
            limit: result.limit,
            total: result.total,
            totalPages: result.totalPages,
        });
    }, "AdminTelemetryController.listGroups");

    groupDetail = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const detail = await telemetryService.getGroupDetail(req.params.id);
        if (!detail) {
            throw new ApiError({ statusCode: 404, message: "مجموعة الأخطاء غير موجودة", code: "not_found" });
        }
        ApiResponse.success({ res, data: detail });
    }, "AdminTelemetryController.groupDetail");

    /**
     * Downloads the raw `IngestFinding[]` array as a file — this feeds
     * `bun run scripts/diagnostics/run.ts --ingest <file>` directly, so the response body IS the
     * file content (no `ApiResponse.success` envelope), with a `Content-Disposition` header so a
     * browser save-as gets a sensible filename.
     */
    exportLedger = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const groupIdsParam = req.query.groupIds as string | undefined;
        const groupIds = groupIdsParam
            ? groupIdsParam
                  .split(",")
                  .map((id) => id.trim())
                  .filter(Boolean)
            : undefined;

        const findings = await telemetryService.exportForIngestLedger(groupIds);

        res.setHeader("Content-Disposition", 'attachment; filename="telemetry-export.json"');
        res.json(findings);
    }, "AdminTelemetryController.exportLedger");
}

export const adminTelemetryController = new AdminTelemetryController();
export default adminTelemetryController;
