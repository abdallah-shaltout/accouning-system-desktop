import { and, eq, isNull } from "drizzle-orm";
import { BaseService, type DbOrTx } from "@@shared/core/service.core";
import { db } from "@@config/database/client";
import { user, type NewUser, type User } from "./user.schema";

/** User is soft-deleted; BaseService gives create/read/update for free. */
class UserService extends BaseService<typeof user> {
    constructor() {
        super(user, { deletedAtColumn: "deletedAt" });
    }

    async findByPhone(phone: string, tx?: DbOrTx): Promise<User | null> {
        const client = tx ?? db;
        const [row] = await (client as any)
            .select()
            .from(user)
            .where(and(eq(user.phone, phone), isNull(user.deletedAt)));
        return row ?? null;
    }

    async isPhoneRegistered(phone: string, tx?: DbOrTx): Promise<boolean> {
        const row = await this.findByPhone(phone, tx);
        return row !== null;
    }

    async createUser(data: NewUser, tx?: DbOrTx): Promise<User> {
        return this.createDocument(data, tx);
    }

    /** Active (non-deleted) owners in an org — used to block removing the last owner. */
    async countOwners(orgId: string, tx?: DbOrTx): Promise<number> {
        return this.countDocuments({ orgId, role: "owner" }, tx);
    }
}

export const userService = new UserService();
export default userService;
