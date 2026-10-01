import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { deviceService } from "@@device/device.service";
import { activationService } from "@@activation/activation.service";
import { licenseService } from "@@license/license.service";
import { diagnosticsService } from "@@diagnostics/diagnostics.service";
import type { ActivateDeviceBody, HeartbeatBody, RegisterDeviceBody } from "./device.validation";

/**
 * `/api/device` surface: register (public), activate (device-authenticated), heartbeat
 * (device-authenticated). Thin — every rule lives in device/activation/license services.
 */
class DeviceController {
    register = AsyncHandler(async (req: Request, res: Response) => {
        const body = req.body as RegisterDeviceBody;
        const result = await deviceService.register(body);
        ApiResponse.success({
            res,
            statusCode: 201,
            message: "تم تسجيل الجهاز بنجاح",
            data: result,
        });
    }, "DeviceController.register");

    activate = AsyncHandler(async (req: Request, res: Response) => {
        if (!req.device) {
            throw new ApiError({ statusCode: 401, message: "بيانات اعتماد الجهاز مطلوبة", code: "unauthorized" });
        }
        const body = req.body as ActivateDeviceBody;
        const result = await activationService.activate(req.device.id, req.device.role, body);
        ApiResponse.success({ res, message: "تم تفعيل الجهاز بنجاح", data: result });
    }, "DeviceController.activate");

    heartbeat = AsyncHandler(async (req: Request, res: Response) => {
        if (!req.device) {
            throw new ApiError({ statusCode: 401, message: "بيانات اعتماد الجهاز مطلوبة", code: "unauthorized" });
        }
        const body = req.body as HeartbeatBody;

        await deviceService.touchHeartbeat(req.device.id, body);

        const needsReissue = await licenseService.needsReissue(req.device.id);
        const license = needsReissue ? await licenseService.issueFor(req.device.id) : undefined;
        const freePolicy = await licenseService.issueFreePolicy();

        // Phase C (diagnostics domain): pending, non-expired requests for this device, mapped to
        // just {id} per docs/05-api-spec.md's heartbeat response shape.
        const pendingDiagnostics = await diagnosticsService.listPending(req.device.id);

        ApiResponse.success({
            res,
            data: {
                ...(license !== undefined ? { license } : {}),
                freePolicy,
                serverTime: Date.now(),
                diagnosticsRequests: pendingDiagnostics.map((r) => ({ id: r.id })),
            },
        });
    }, "DeviceController.heartbeat");
}

export const deviceController = new DeviceController();
export default deviceController;
