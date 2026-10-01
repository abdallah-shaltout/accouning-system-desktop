import type { Request, Response, NextFunction } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { creditService } from "@@credit/credit.service";
import type { ConsumeCreditBody } from "./device.credit.validation";

/**
 * `POST /device/credits/consume` is NOT wrapped with the generic `idempotency()` middleware
 * (shared/middleware/idempotency): that middleware replays a whole HTTP response and is scoped to
 * request/response idempotency, which is coarser than what's needed here. creditService.consume
 * already implements ledger-level idempotency keyed on the same header value (a `credit_ledger`
 * row keyed on `idempotencyKey`, checked inside one transaction) — wrapping this route with both
 * would be redundant and could conflict (e.g. the generic middleware caching a 429 permanently).
 * So this controller reads the header itself and passes it straight into the service.
 */
export class DeviceCreditController {
    getUsage = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const usage = await creditService.getUsage(req.device?.orgId ?? null);
        ApiResponse.success({ res, data: usage });
    }, "DeviceCreditController.getUsage");

    consume = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const idempotencyKey = req.headers["idempotency-key"];
        if (!idempotencyKey || typeof idempotencyKey !== "string") {
            throw new ApiError({
                statusCode: 400,
                message: "رأس Idempotency-Key مطلوب",
                code: "validation_failed",
            });
        }

        const { feature } = req.body as ConsumeCreditBody;
        const result = await creditService.consume(req.device?.orgId ?? null, feature, idempotencyKey, req.device!.id);
        ApiResponse.success({ res, data: result });
    }, "DeviceCreditController.consume");
}

export const deviceCreditController = new DeviceCreditController();
export default deviceCreditController;
