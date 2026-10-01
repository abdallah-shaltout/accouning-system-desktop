import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import { ApiError } from "@@shared/middleware/error/apiError";
import { releaseService } from "@@release/release.service";
import type { LatestReleaseQuery } from "./device.release.validation";

/**
 * `/api/device/releases` surface. `GET /latest` mirrors the Tauri updater's expected JSON shape
 * exactly (snake_case `pub_date`, `platforms["windows-x86_64"]`) — this response is consumed
 * directly by the desktop updater plugin, not by our own client code, so its shape is fixed by an
 * external contract, not our usual camelCase convention.
 */
class DeviceReleaseController {
    latest = AsyncHandler(async (req: Request, res: Response) => {
        if (!req.device) {
            throw new ApiError({ statusCode: 401, message: "بيانات اعتماد الجهاز مطلوبة", code: "unauthorized" });
        }

        const { current, channel } = req.query as unknown as LatestReleaseQuery;

        const found = await releaseService.getLatestForDevice({
            channel,
            currentVersion: current,
            deviceId: req.device.id,
        });

        if (!found) {
            res.status(204).end();
            return;
        }

        res.status(200).json({
            version: found.version,
            notes: found.notes ?? "",
            pub_date: (found.publishedAt ?? found.createdAt).toISOString(),
            platforms: {
                "windows-x86_64": {
                    url: found.url,
                    signature: found.signature,
                },
            },
            mandatory: found.isMandatory,
        });
    }, "DeviceReleaseController.latest");
}

export const deviceReleaseController = new DeviceReleaseController();
export default deviceReleaseController;
