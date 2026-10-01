import type { Request, Response, NextFunction } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { creditService } from "@@credit/credit.service";

export class PortalCreditController {
    getUsage = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const usage = await creditService.getUsage(req.orgId ?? null);
        ApiResponse.success({ res, data: usage });
    }, "PortalCreditController.getUsage");
}

export const portalCreditController = new PortalCreditController();
export default portalCreditController;
