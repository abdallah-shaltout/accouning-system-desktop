import type { Request, Response, NextFunction } from "express";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { device } from "@@device/device.schema";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { telemetryService } from "@@telemetry/telemetry.service";
import type { ErrorsBody } from "./device.telemetry.validation";

/**
 * `req.device` (set by `DeviceRequiredAuth`) only carries id/orgId/terminalId/role — it does not
 * carry `telemetryEnabled` (see `shared/types/Express.type.d.ts`), so the toggle is looked up here
 * with one quick query before doing any ingestion work.
 */
export class DeviceTelemetryController {
    reportErrors = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const deviceId = req.device!.id;

        const [row] = await db.select({ telemetryEnabled: device.telemetryEnabled }).from(device).where(eq(device.id, deviceId));

        if (!row || !row.telemetryEnabled) {
            // A device respecting the user's privacy toggle isn't an error — 200, no writes.
            ApiResponse.success({ res, message: "متوقف" });
            return;
        }

        const batch = req.body as ErrorsBody;

        // `telemetryService.ingestErrors` takes one appVersion per call; the wire format
        // (docs/05-api-spec.md's Device table) carries `appVersion` per item, since a batch can
        // span an app update mid-session. Group by appVersion so each sub-batch is ingested under
        // its own version without changing the service's per-call contract.
        const byVersion = new Map<string, typeof batch>();
        for (const item of batch) {
            const group = byVersion.get(item.appVersion) ?? [];
            group.push(item);
            byVersion.set(item.appVersion, group);
        }

        for (const [appVersion, items] of byVersion) {
            await telemetryService.ingestErrors(deviceId, appVersion, items);
        }

        ApiResponse.success({ res, message: "تم استلام بيانات القياس" });
    }, "DeviceTelemetryController.reportErrors");
}

export const deviceTelemetryController = new DeviceTelemetryController();
export default deviceTelemetryController;
