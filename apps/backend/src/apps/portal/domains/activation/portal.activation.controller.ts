import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { activationService } from "@@activation/activation.service";
import type { ApproveActivationBody } from "./portal.activation.validation";

/** Portal side of the PKCE-style device activation handshake. See B5/05-api-spec.md. */
class PortalActivationController {
    approve = AsyncHandler(async (req: Request, res: Response) => {
        const body = req.body as ApproveActivationBody;
        const result = await activationService.approve({
            challenge: body.challenge,
            state: body.state,
            terminalId: body.terminalId,
            deviceName: body.deviceName,
            orgId: req.orgId!,
            userId: req.user!.id,
        });
        ApiResponse.success({ res, statusCode: 201, message: "تم إنشاء رمز التفعيل", data: result });
    }, "PortalActivationController.approve");
}

export const portalActivationController = new PortalActivationController();
export default portalActivationController;
