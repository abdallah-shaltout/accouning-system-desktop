import { randomUUID } from "crypto";
import { BaseService, type PaginatedResult } from "@@shared/core/service.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { isR2Configured, uploadObject } from "@@shared/storage/r2";
import { feedback, type Feedback } from "./feedback.schema";

export interface SubmitFeedbackInput {
    deviceId: string;
    orgId: string | null;
    message: string;
    screenshotBuffer?: Buffer | null;
    screenshotMimeType?: string;
    bundleBuffer?: Buffer | null;
    bundleMimeType?: string;
}

/** `domains/feedback` — device-submitted feedback (message + optional screenshot/bundle) → admin inbox (C5). */
export class FeedbackService extends BaseService<typeof feedback> {
    constructor() {
        super(feedback);
    }

    /**
     * Uploads any attached files to R2 first, then inserts the row. Storage is only required when a
     * file is actually attached — plain-text feedback must keep working even without R2 configured.
     */
    async submit(input: SubmitFeedbackInput): Promise<Feedback> {
        const hasUpload = Boolean(input.screenshotBuffer) || Boolean(input.bundleBuffer);
        if (hasUpload && !isR2Configured()) {
            throw new ApiError({ statusCode: 503, message: "رفع الملفات غير متاح حاليًا — التخزين غير مُهيأ", code: "storage_not_configured" });
        }

        const groupId = randomUUID();
        let screenshotKey: string | null = null;
        let bundleKey: string | null = null;

        if (input.screenshotBuffer) {
            screenshotKey = `feedback/${groupId}/screenshot.png`;
            await uploadObject("private", screenshotKey, input.screenshotBuffer, input.screenshotMimeType ?? "image/png");
        }

        if (input.bundleBuffer) {
            bundleKey = `feedback/${groupId}/bundle.zip`;
            await uploadObject("private", bundleKey, input.bundleBuffer, input.bundleMimeType ?? "application/zip");
        }

        return this.createDocument({
            deviceId: input.deviceId,
            orgId: input.orgId,
            message: input.message,
            screenshotKey,
            bundleKey,
            status: "new",
        });
    }

    async listForAdmin(opts: { status?: Feedback["status"]; page?: number; limit?: number }): Promise<PaginatedResult<Feedback>> {
        return this.readDocument({
            query: { page: opts.page, limit: opts.limit, sort: "-createdAt" },
            reqFilter: { status: opts.status },
        });
    }

    async updateStatus(feedbackId: string, status: Feedback["status"]): Promise<Feedback> {
        return this.updateDocument({ id: feedbackId, data: { status } });
    }
}

export const feedbackService = new FeedbackService();
export default feedbackService;
