import { and, asc, desc, ilike, isNull, or, sql } from "drizzle-orm";
import { BaseService, type DbOrTx, type PaginatedResult } from "@@shared/core/service.core";
import { db } from "@@config/database/client";
import { organization, type Organization } from "./organization.schema";

/** Organization is soft-deleted; BaseService gives create/read/update for free. */
class OrganizationService extends BaseService<typeof organization> {
    constructor() {
        super(organization, { deletedAtColumn: "deletedAt" });
    }

    async createOrganization(
        data: { name: string; phone: string; governorate?: string; area?: string },
        tx?: DbOrTx,
    ) {
        return this.createDocument(data, tx);
    }

    /**
     * Admin org search: BaseService.readDocument only supports a single-field `search`+`searchBy`
     * pair, but the B1 admin listing needs an OR match across name and phone in one `?search=`.
     */
    async searchOrganizations(query: {
        page?: number;
        limit?: number;
        sort?: string;
        search?: string;
    }): Promise<PaginatedResult<Organization>> {
        const page = Math.max(1, Number(query.page) || 1);
        const limit = Math.min(100, Math.max(1, Number(query.limit) || 20));
        const offset = (page - 1) * limit;

        const clauses = [isNull(organization.deletedAt)];
        if (query.search) {
            clauses.push(or(ilike(organization.name, `%${query.search}%`), ilike(organization.phone, `%${query.search}%`))!);
        }
        const whereClause = and(...clauses);

        const orderColumn = query.sort ? String(query.sort).replace(/^-/, "") : "createdAt";
        const orderDir = String(query.sort ?? "").startsWith("-") ? desc : asc;
        const orderExpr = (organization as any)[orderColumn] ?? organization.createdAt;

        const [rows, totalRow] = await Promise.all([
            db
                .select()
                .from(organization)
                .where(whereClause)
                .orderBy(orderDir(orderExpr))
                .limit(limit)
                .offset(offset),
            db.select({ count: sql<number>`count(*)::int` }).from(organization).where(whereClause),
        ]);

        const total = totalRow[0]?.count ?? 0;

        return {
            data: rows,
            page,
            limit,
            total,
            totalPages: Math.max(1, Math.ceil(total / limit)),
        };
    }
}

export const organizationService = new OrganizationService();
export default organizationService;
