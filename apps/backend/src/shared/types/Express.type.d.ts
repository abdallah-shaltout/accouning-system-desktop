import type { Audience } from "@@shared/utils/Token";

declare global {
    namespace Express {
        interface Request {
            admin?: { id: string; email: string; role: "owner" | "support" | "finance" };
            user?: { id: string; orgId: string; role: "owner" | "member" };
            orgId?: string;
            device?: { id: string; orgId: string | null; terminalId: string; role: "main" | "terminal" };
            tokenAud?: Audience;
        }
    }
}

export {};
