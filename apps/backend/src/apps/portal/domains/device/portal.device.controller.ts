import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { deviceService } from "@@device/device.service";
import { licenseService } from "@@license/license.service";
import type { RenameDeviceBody } from "./portal.device.validation";

/** Portal device management, scoped to `req.orgId`. Never leaks another org's device (404). */
class PortalDeviceController {
    list = AsyncHandler(async (req: Request, res: Response) => {
        const devices = await deviceService.listForOrg(req.orgId!);
        ApiResponse.success({ res, data: devices });
    }, "PortalDeviceController.list");

    rename = AsyncHandler(async (req: Request, res: Response) => {
        const { name } = req.body as RenameDeviceBody;
        const updated = await deviceService.renameForOrg(req.orgId!, req.params.id, name);
        ApiResponse.success({ res, message: "تم تحديث اسم الجهاز", data: updated });
    }, "PortalDeviceController.rename");

    deactivate = AsyncHandler(async (req: Request, res: Response) => {
        const device = await deviceService.requireDeviceById(req.params.id);
        if (device.orgId !== req.orgId) {
            throw new ApiError({ statusCode: 404, message: "الجهاز غير موجود", code: "not_found" });
        }

        await deviceService.withTx(async (tx) => {
            await deviceService.deactivate(device.id, tx);
            await licenseService.revokeForDevice(device.id, tx);
        });

        ApiResponse.success({ res, message: "تم إلغاء تفعيل الجهاز" });
    }, "PortalDeviceController.deactivate");
}

export const portalDeviceController = new PortalDeviceController();
export default portalDeviceController;
