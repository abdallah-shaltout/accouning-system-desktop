import type { Request, Response, NextFunction } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { adminActivityService } from "@@adminActivity/adminActivity.service";
import { planService } from "@@plan/plan.service";

/**
 * Plan versions have their own lifecycle (draft -> published -> retired) enforced entirely in
 * `plan.service.ts`. This controller stays thin: parse request, call the named service method,
 * record the admin write, respond. It does not extend `AdminBaseController` because plan versions
 * are never created/updated/deleted through generic CRUD — see plan.service.ts.
 */
export class AdminPlanController {
    private readonly targetType = "plan_version";

    listPlans = AsyncHandler(async (_req: Request, res: Response, _next: NextFunction) => {
        const plans = await planService.listAllWithVersions();
        ApiResponse.success({ res, data: plans });
    }, "AdminPlanController.listPlans");

    createDraftVersion = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const { priceMonthly, priceYearly, entitlements } = req.body as {
            priceMonthly: number;
            priceYearly: number;
            entitlements: unknown;
        };

        const created = await planService.createDraftVersion(
            req.params.id,
            { priceMonthly: BigInt(priceMonthly), priceYearly: BigInt(priceYearly), entitlements },
            req.admin!.id,
        );

        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "create",
            targetType: this.targetType,
            targetId: created.id,
            after: created,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });

        ApiResponse.success({ res, statusCode: 201, message: "تم إنشاء مسودة إصدار جديد", data: created });
    }, "AdminPlanController.createDraftVersion");

    editDraft = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const before = await planService.requireVersionById(req.params.id);
        const { priceMonthly, priceYearly, entitlements } = req.body as {
            priceMonthly?: number;
            priceYearly?: number;
            entitlements?: unknown;
        };

        const updated = await planService.editDraft(
            req.params.id,
            {
                priceMonthly: priceMonthly !== undefined ? BigInt(priceMonthly) : undefined,
                priceYearly: priceYearly !== undefined ? BigInt(priceYearly) : undefined,
                entitlements,
            },
            req.admin!.id,
        );

        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "update",
            targetType: this.targetType,
            targetId: updated.id,
            before,
            after: updated,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });

        ApiResponse.success({ res, message: "تم تحديث المسودة بنجاح", data: updated });
    }, "AdminPlanController.editDraft");

    publishVersion = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const before = await planService.requireVersionById(req.params.id);
        const updated = await planService.publishVersion(req.params.id, req.admin!.id);

        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "publish",
            targetType: this.targetType,
            targetId: updated.id,
            before,
            after: updated,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });

        ApiResponse.success({ res, message: "تم نشر الإصدار بنجاح", data: updated });
    }, "AdminPlanController.publishVersion");

    retireVersion = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const before = await planService.requireVersionById(req.params.id);
        const updated = await planService.retireVersion(req.params.id, req.admin!.id);

        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "retire",
            targetType: this.targetType,
            targetId: updated.id,
            before,
            after: updated,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });

        ApiResponse.success({ res, message: "تم إيقاف الإصدار بنجاح", data: updated });
    }, "AdminPlanController.retireVersion");
}

export const adminPlanController = new AdminPlanController();
export default adminPlanController;
