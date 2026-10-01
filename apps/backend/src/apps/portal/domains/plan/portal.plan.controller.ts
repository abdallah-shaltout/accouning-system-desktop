import type { Request, Response, NextFunction } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { planService } from "@@plan/plan.service";

export class PortalPlanController {
    listPublished = AsyncHandler(async (_req: Request, res: Response, _next: NextFunction) => {
        const plans = await planService.listPublishedForPortal();
        ApiResponse.success({ res, data: plans });
    }, "PortalPlanController.listPublished");
}

export const portalPlanController = new PortalPlanController();
export default portalPlanController;
