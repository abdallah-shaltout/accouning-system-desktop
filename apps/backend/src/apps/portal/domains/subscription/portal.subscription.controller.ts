import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { subscriptionService } from "@@subscription/subscription.service";

/**
 * Portal subscription surface: everything is scoped to `req.orgId` (the caller's own org), never a
 * client-sent org id.
 */
class PortalSubscriptionController {
    getCurrent = AsyncHandler(async (req: Request, res: Response) => {
        const orgId = req.orgId!;
        const [occupying, effective] = await Promise.all([
            subscriptionService.findOccupyingSubscription(orgId),
            subscriptionService.getEffectiveEntitlements(orgId),
        ]);

        const events = occupying ? await subscriptionService.recentEvents(occupying.id, 20) : [];

        ApiResponse.success({
            res,
            data: {
                subscription: occupying,
                plan: {
                    planKey: effective.planKey,
                    planVersionId: effective.planVersionId,
                    entitlements: effective.entitlements,
                    currentPeriodEnd: effective.currentPeriodEnd,
                },
                events,
            },
        });
    }, "PortalSubscriptionController.getCurrent");

    checkout = AsyncHandler(async (req: Request, res: Response) => {
        const { planVersionId, interval } = req.body as { planVersionId: string; interval: "month" | "year" };
        const result = await subscriptionService.startCheckout(req.orgId!, planVersionId, interval, req.user!.id);
        ApiResponse.success({
            res,
            statusCode: 201,
            message: "تم بدء عملية الاشتراك — أكمل السداد لتفعيله",
            data: result,
        });
    }, "PortalSubscriptionController.checkout");

    cancel = AsyncHandler(async (req: Request, res: Response) => {
        const current = await subscriptionService.findOccupyingSubscription(req.orgId!);
        if (!current) {
            return ApiResponse.fail({ res, statusCode: 404, message: "لا يوجد اشتراك نشط", code: "not_found" });
        }
        const updated = await subscriptionService.cancelAtPeriodEnd(current.id, req.user!.id);
        ApiResponse.success({ res, message: "سيتم إلغاء الاشتراك في نهاية الفترة الحالية", data: updated });
    }, "PortalSubscriptionController.cancel");

    resume = AsyncHandler(async (req: Request, res: Response) => {
        const current = await subscriptionService.findOccupyingSubscription(req.orgId!);
        if (!current) {
            return ApiResponse.fail({ res, statusCode: 404, message: "لا يوجد اشتراك نشط", code: "not_found" });
        }
        const updated = await subscriptionService.resume(current.id, req.user!.id);
        ApiResponse.success({ res, message: "تم التراجع عن الإلغاء", data: updated });
    }, "PortalSubscriptionController.resume");
}

export const portalSubscriptionController = new PortalSubscriptionController();
export default portalSubscriptionController;
