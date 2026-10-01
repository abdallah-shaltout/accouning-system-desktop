import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { organizationService } from "@@organization/organization.service";

/**
 * Portal organization surface: the caller only ever sees/edits their own org (req.orgId), never a
 * client-sent id. Phone and status are admin/system-only, so they are read-only here even though
 * they're returned in GET.
 */
class PortalOrganizationController {
    getMine = AsyncHandler(async (req: Request, res: Response) => {
        const org = await organizationService.requireDocumentById(req.orgId!, "المنشأة غير موجودة");
        ApiResponse.success({
            res,
            data: {
                id: org.id,
                name: org.name,
                phone: org.phone,
                governorate: org.governorate,
                area: org.area,
                status: org.status,
            },
        });
    }, "PortalOrganizationController.getMine");

    updateMine = AsyncHandler(async (req: Request, res: Response) => {
        const { name, governorate, area } = req.body as {
            name?: string;
            governorate?: string;
            area?: string;
        };

        const updated = await organizationService.updateDocument({
            id: req.orgId!,
            data: { name, governorate, area },
        });

        ApiResponse.success({
            res,
            message: "تم تحديث بيانات المنشأة بنجاح",
            data: {
                id: updated.id,
                name: updated.name,
                phone: updated.phone,
                governorate: updated.governorate,
                area: updated.area,
                status: updated.status,
            },
        });
    }, "PortalOrganizationController.updateMine");
}

export const portalOrganizationController = new PortalOrganizationController();
export default portalOrganizationController;
