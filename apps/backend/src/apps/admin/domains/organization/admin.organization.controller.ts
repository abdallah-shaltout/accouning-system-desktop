import type { Request, Response } from "express";
import { AdminBaseController } from "@/admin/core/admin.controller.core";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { organizationService } from "@@organization/organization.service";

/**
 * Admin organization surface. List/detail are plain reads (not audited); PATCH (notes/status) goes
 * through AdminBaseController so every write lands in admin_activity per docs/06-security.md.
 */
class AdminOrganizationController extends AdminBaseController<typeof organizationService> {
    constructor() {
        super(organizationService, "المنشأة", "organization");
    }

    readDocument = AsyncHandler(async (req: Request, res: Response) => {
        const result = await organizationService.searchOrganizations({
            page: req.query.page as any,
            limit: req.query.limit as any,
            sort: req.query.sort as string | undefined,
            search: req.query.search as string | undefined,
        });
        ApiResponse.success({
            res,
            data: result.data,
            page: result.page,
            limit: result.limit,
            total: result.total,
            totalPages: result.totalPages,
        });
    }, "AdminOrganizationController.readDocument");

    /**
     * TODO: these null/empty placeholders should be filled in once domains/subscription,
     * domains/device, domains/payment, domains/credit exist — this endpoint's shape must not
     * change, only these fields need real queries added (B3/B5/B4/B7 land them separately).
     */
    detail = AsyncHandler(async (req: Request, res: Response) => {
        const org = await organizationService.requireDocumentById(req.params.id, "المنشأة غير موجودة");

        ApiResponse.success({
            res,
            data: {
                ...org,
                subscription: null,
                devices: [] as unknown[],
                payments: [] as unknown[],
                credits: null,
            },
        });
    }, "AdminOrganizationController.detail");
}

export const adminOrganizationController = new AdminOrganizationController();
export default adminOrganizationController;
