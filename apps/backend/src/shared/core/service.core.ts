import { and, asc, desc, eq, ilike, isNull, sql, type SQL } from "drizzle-orm";
import type { PgTable } from "drizzle-orm/pg-core";
import { db } from "@@config/database/client";
import { ApiError } from "@@shared/middleware/error/apiError";

type Tx = Parameters<Parameters<typeof db.transaction>[0]>[0];
export type DbOrTx = typeof db | Tx;

export interface ReadDocumentOptions {
    query?: Record<string, any>;
    reqFilter?: Record<string, any>;
}

export interface PaginatedResult<T> {
    data: T[];
    page: number;
    limit: number;
    total: number;
    totalPages: number;
}

/**
 * Ported from the reference `BaseService` (Mongoose) to Drizzle, keeping the same method names
 * and return shapes so domain services read the same way. `deletedAtColumn`, when present, makes
 * deleteDocument a soft delete and read/count exclude soft-deleted rows automatically.
 */
export class BaseService<TTable extends PgTable & { id: any }> {
    protected table: TTable;
    protected deletedAtColumn?: keyof TTable;

    constructor(table: TTable, opts: { deletedAtColumn?: keyof TTable } = {}) {
        this.table = table;
        this.deletedAtColumn = opts.deletedAtColumn;
    }

    protected notDeletedClause(): SQL | undefined {
        if (!this.deletedAtColumn) return undefined;
        return isNull((this.table as any)[this.deletedAtColumn]);
    }

    protected client(tx?: DbOrTx): DbOrTx {
        return tx ?? db;
    }

    async withTx<T>(fn: (tx: Tx) => Promise<T>): Promise<T> {
        return db.transaction(fn);
    }

    async createDocument(data: Partial<TTable["$inferInsert"]>, tx?: DbOrTx): Promise<TTable["$inferSelect"]> {
        const client = this.client(tx);
        const [row] = await (client as any).insert(this.table).values(data).returning();
        return row;
    }

    async readDocument({
        query = {},
        reqFilter = {},
    }: ReadDocumentOptions = {}): Promise<PaginatedResult<TTable["$inferSelect"]>> {
        const page = Math.max(1, Number(query.page) || 1);
        const limit = Math.min(100, Math.max(1, Number(query.limit) || 20));
        const offset = (page - 1) * limit;

        const clauses: SQL[] = [];
        const notDeleted = this.notDeletedClause();
        if (notDeleted) clauses.push(notDeleted);

        for (const [key, value] of Object.entries(reqFilter)) {
            if (value === undefined) continue;
            clauses.push(eq((this.table as any)[key], value));
        }

        if (query.search && query.searchBy) {
            clauses.push(ilike((this.table as any)[query.searchBy as string], `%${query.search}%`));
        }

        const whereClause = clauses.length ? and(...clauses) : undefined;

        const orderColumn = query.sort ? String(query.sort).replace(/^-/, "") : "createdAt";
        const orderDir = String(query.sort ?? "").startsWith("-") ? desc : asc;
        const orderExpr = (this.table as any)[orderColumn] ?? (this.table as any).createdAt;

        const [rows, totalRow] = await Promise.all([
            (db as any)
                .select()
                .from(this.table)
                .where(whereClause)
                .orderBy(orderExpr ? orderDir(orderExpr) : undefined)
                .limit(limit)
                .offset(offset),
            (db as any)
                .select({ count: sql<number>`count(*)::int` })
                .from(this.table)
                .where(whereClause),
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

    async readDocumentById(id: string, tx?: DbOrTx): Promise<TTable["$inferSelect"] | null> {
        const client = this.client(tx);
        const clauses = [eq((this.table as any).id, id)];
        const notDeleted = this.notDeletedClause();
        if (notDeleted) clauses.push(notDeleted);

        const [row] = await (client as any)
            .select()
            .from(this.table)
            .where(and(...clauses));
        return row ?? null;
    }

    async requireDocumentById(id: string, notFoundMessage = "السجل غير موجود", tx?: DbOrTx): Promise<TTable["$inferSelect"]> {
        const row = await this.readDocumentById(id, tx);
        if (!row) {
            throw new ApiError({ statusCode: 404, message: notFoundMessage, code: "not_found" });
        }
        return row;
    }

    async updateDocument(
        { id, data }: { id: string; data: Partial<TTable["$inferInsert"]> },
        tx?: DbOrTx,
    ): Promise<TTable["$inferSelect"]> {
        const client = this.client(tx);
        const [row] = await (client as any)
            .update(this.table)
            .set({ ...data, updatedAt: new Date() } as any)
            .where(eq((this.table as any).id, id))
            .returning();
        if (!row) {
            throw new ApiError({ statusCode: 404, message: "السجل غير موجود", code: "not_found" });
        }
        return row;
    }

    async deleteDocument({ id }: { id: string }, tx?: DbOrTx): Promise<string | null> {
        const client = this.client(tx);
        if (this.deletedAtColumn) {
            const [row] = await (client as any)
                .update(this.table)
                .set({ [this.deletedAtColumn]: new Date() } as any)
                .where(eq((this.table as any).id, id))
                .returning({ id: (this.table as any).id });
            return row?.id ?? null;
        }
        const [row] = await (client as any)
            .delete(this.table)
            .where(eq((this.table as any).id, id))
            .returning({ id: (this.table as any).id });
        return row?.id ?? null;
    }

    async countDocuments(reqFilter: Record<string, any> = {}, tx?: DbOrTx): Promise<number> {
        const client = this.client(tx);
        const clauses: SQL[] = [];
        const notDeleted = this.notDeletedClause();
        if (notDeleted) clauses.push(notDeleted);
        for (const [key, value] of Object.entries(reqFilter)) {
            if (value === undefined) continue;
            clauses.push(eq((this.table as any)[key], value));
        }
        const whereClause = clauses.length ? and(...clauses) : undefined;
        const [row] = await (client as any)
            .select({ count: sql<number>`count(*)::int` })
            .from(this.table)
            .where(whereClause);
        return row?.count ?? 0;
    }

    async isExists(reqFilter: Record<string, any>, tx?: DbOrTx): Promise<boolean> {
        const count = await this.countDocuments(reqFilter, tx);
        return count > 0;
    }
}

export default BaseService;
