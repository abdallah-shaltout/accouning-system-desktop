import { BaseService } from "@@shared/core/service.core";
import { adminActivity } from "./adminActivity.schema";
import type { DbOrTx } from "@@shared/core/service.core";

class AdminActivityService extends BaseService<typeof adminActivity> {
    constructor() {
        super(adminActivity);
    }

    async log(
        entry: {
            adminId: string;
            action: string;
            targetType: string;
            targetId: string;
            before?: unknown;
            after?: unknown;
            ip?: string;
            userAgent?: string | string[];
        },
        tx?: DbOrTx,
    ) {
        return this.createDocument(
            {
                adminId: entry.adminId,
                action: entry.action,
                targetType: entry.targetType,
                targetId: entry.targetId,
                before: entry.before as any,
                after: entry.after as any,
                ip: entry.ip,
                userAgent: Array.isArray(entry.userAgent) ? entry.userAgent[0] : entry.userAgent,
            },
            tx,
        );
    }
}

export const adminActivityService = new AdminActivityService();
export default adminActivityService;
