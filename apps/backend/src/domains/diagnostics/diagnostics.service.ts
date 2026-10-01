import { and, eq, isNull, lt, gt } from "drizzle-orm";
import { db } from "@@config/database/client";
import { BaseService, type DbOrTx, type PaginatedResult } from "@@shared/core/service.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { isR2Configured, uploadObject, presignDownloadUrl } from "@@shared/storage/r2";
import { adminActivityService } from "@@adminActivity/adminActivity.service";
import { device } from "@@device/device.schema";
import { diagnosticsRequest, type DiagnosticsRequest } from "./diagnostics.schema";

const MAX_BUNDLE_BYTES = 20 * 1024 * 1024;
const REQUEST_TTL_MS = 14 * 24 * 60 * 60 * 1000;

export interface CreateDiagnosticsRequestInput {
    deviceIdOrOrgId: string;
    kind: "device" | "org";
    requestedByAdminId: string;
}

/**
 * `domains/diagnostics` — admin-initiated pull of a desktop diagnostics bundle (C4). The server
 * never reaches into accounting data: it only ever stores/serves the zip the desktop itself
 * packages (logs + app version + OS + redacted settings, per docs/06-security.md).
 */
export class DiagnosticsService extends BaseService<typeof diagnosticsRequest> {
    constructor() {
        super(diagnosticsRequest);
    }

    async createRequest(input: CreateDiagnosticsRequestInput): Promise<DiagnosticsRequest[]> {
        const expiresAt = new Date(Date.now() + REQUEST_TTL_MS);

        if (input.kind === "device") {
            const [dev] = await db.select().from(device).where(eq(device.id, input.deviceIdOrOrgId));
            if (!dev) {
                throw new ApiError({ statusCode: 404, message: "الجهاز غير موجود", code: "not_found" });
            }

            const created = await this.createDocument({
                deviceId: dev.id,
                orgId: dev.orgId,
                requestedBy: input.requestedByAdminId,
                status: "pending",
                expiresAt,
            });
            return [created];
        }

        const activeDevices = await db
            .select()
            .from(device)
            .where(and(eq(device.orgId, input.deviceIdOrOrgId), isNull(device.revokedAt)));

        if (activeDevices.length === 0) {
            throw new ApiError({ statusCode: 404, message: "لا توجد أجهزة نشطة لهذه المنشأة", code: "not_found" });
        }

        const created: DiagnosticsRequest[] = [];
        for (const dev of activeDevices) {
            const row = await this.createDocument({
                deviceId: dev.id,
                orgId: dev.orgId,
                requestedBy: input.requestedByAdminId,
                status: "pending",
                expiresAt,
            });
            created.push(row);
        }
        return created;
    }

    private async requireOwnedPendingRequest(requestId: string, deviceId: string): Promise<DiagnosticsRequest> {
        const [row] = await db.select().from(diagnosticsRequest).where(eq(diagnosticsRequest.id, requestId));

        // Don't leak existence to a different device: an unknown id and a request belonging to
        // another device both 404 the same way.
        if (!row || row.deviceId !== deviceId) {
            throw new ApiError({ statusCode: 404, message: "طلب التشخيص غير موجود", code: "not_found" });
        }

        if (row.status !== "pending") {
            throw new ApiError({ statusCode: 409, message: "تمت معالجة هذا الطلب من قبل", code: "conflict" });
        }

        return row;
    }

    async fulfillWithUpload(requestId: string, deviceId: string, fileBuffer: Buffer, sizeBytes: number): Promise<DiagnosticsRequest> {
        const request = await this.requireOwnedPendingRequest(requestId, deviceId);

        if (sizeBytes > MAX_BUNDLE_BYTES) {
            throw new ApiError({ statusCode: 422, message: "حجم الملف أكبر من الحد المسموح به (20 ميجابايت)", code: "too_large" });
        }

        const [dev] = await db.select().from(device).where(eq(device.id, deviceId));
        if (!dev) {
            throw new ApiError({ statusCode: 404, message: "الجهاز غير موجود", code: "not_found" });
        }
        // Defensive: a well-behaved client never calls this while the toggle is off, but the server
        // must not trust the client (CLAUDE.md security rules).
        if (!dev.diagnosticsAllowed) {
            throw new ApiError({ statusCode: 403, message: "تم إيقاف إرسال بيانات التشخيص لهذا الجهاز", code: "forbidden" });
        }

        if (!isR2Configured()) {
            throw new ApiError({ statusCode: 503, message: "رفع الملفات غير متاح حاليًا — التخزين غير مُهيأ", code: "storage_not_configured" });
        }

        const fileKey = `diagnostics/${requestId}.zip`;
        await uploadObject("private", fileKey, fileBuffer, "application/zip");

        return this.updateDocument({
            id: request.id,
            data: {
                status: "uploaded",
                fileKey,
                sizeBytes,
                fulfilledAt: new Date(),
            },
        });
    }

    async decline(requestId: string, deviceId: string): Promise<DiagnosticsRequest> {
        const request = await this.requireOwnedPendingRequest(requestId, deviceId);

        return this.updateDocument({
            id: request.id,
            data: {
                status: "declined",
                fulfilledAt: new Date(),
            },
        });
    }

    async listPending(deviceId: string): Promise<DiagnosticsRequest[]> {
        return db
            .select()
            .from(diagnosticsRequest)
            .where(and(eq(diagnosticsRequest.deviceId, deviceId), eq(diagnosticsRequest.status, "pending"), gt(diagnosticsRequest.expiresAt, new Date())));
    }

    async listForAdmin(opts: { status?: DiagnosticsRequest["status"]; page?: number; limit?: number }): Promise<PaginatedResult<DiagnosticsRequest>> {
        return this.readDocument({
            query: { page: opts.page, limit: opts.limit, sort: "-createdAt" },
            reqFilter: { status: opts.status },
        });
    }

    async getDownloadUrl(requestId: string, adminId: string, adminIp?: string): Promise<string> {
        const request = await this.requireDocumentById(requestId, "طلب التشخيص غير موجود");

        if (request.status !== "uploaded" || !request.fileKey) {
            throw new ApiError({ statusCode: 409, message: "لم يتم رفع ملف التشخيص بعد", code: "conflict" });
        }

        const url = await presignDownloadUrl("private", request.fileKey, 300);

        await adminActivityService.log({
            adminId,
            action: "download",
            targetType: "diagnostics_request",
            targetId: request.id,
            ip: adminIp,
        });

        return url;
    }

    /** Sets stale `pending` requests to `expired`. Exported for the daily cron (and unit tests). */
    async expireStalePending(tx?: DbOrTx): Promise<number> {
        const client = tx ?? db;
        const rows = await (client as any)
            .update(diagnosticsRequest)
            .set({ status: "expired" })
            .where(and(eq(diagnosticsRequest.status, "pending"), lt(diagnosticsRequest.expiresAt, new Date())))
            .returning({ id: diagnosticsRequest.id });
        return rows.length;
    }
}

export const diagnosticsService = new DiagnosticsService();
export default diagnosticsService;
