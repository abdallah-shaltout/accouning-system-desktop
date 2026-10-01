import type { NextFunction, Request, Response } from "express";
import { ApiError } from "@@shared/middleware/error/apiError";
import { deviceService } from "@@device/device.service";

/**
 * `Authorization: Device <deviceId>.<secret>`. Verifies the secret against the stored hash
 * (constant-time, in `deviceService.verifyCredential`) and rejects a revoked device. Sets
 * `req.device` per `shared/types/Express.type.d.ts` — never re-declared here.
 */
export async function DeviceRequiredAuth(req: Request, _res: Response, next: NextFunction): Promise<void> {
    try {
        const header = req.headers.authorization;
        if (!header || !header.startsWith("Device ")) {
            throw new ApiError({ statusCode: 401, message: "بيانات اعتماد الجهاز مطلوبة", code: "unauthorized" });
        }

        const credential = header.slice("Device ".length);
        const separatorIndex = credential.indexOf(".");
        if (separatorIndex <= 0 || separatorIndex === credential.length - 1) {
            throw new ApiError({ statusCode: 401, message: "بيانات اعتماد الجهاز غير صالحة", code: "unauthorized" });
        }

        const deviceId = credential.slice(0, separatorIndex);
        const secret = credential.slice(separatorIndex + 1);

        const row = await deviceService.verifyCredential(deviceId, secret);

        req.device = { id: row.id, orgId: row.orgId, terminalId: row.terminalId, role: row.role };
        next();
    } catch (err) {
        next(err);
    }
}
