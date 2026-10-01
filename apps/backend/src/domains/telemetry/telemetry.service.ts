import { and, asc, desc, eq, inArray, sql, type SQL } from "drizzle-orm";
import { db } from "@@config/database/client";
import type { PaginatedResult } from "@@shared/core/service.core";
import { errorGroup, errorOccurrence, type ErrorGroup, type ErrorOccurrence } from "./telemetry.schema";

/** Bound per C3 / the data-model doc: the service keeps only the newest 50 occurrence rows per group. */
const MAX_OCCURRENCES_PER_GROUP = 50;

export interface IngestBatchItem {
    fingerprint: string;
    code: string;
    source: string;
    message: string;
    count: number;
    firstSeen: string | Date;
    lastSeen: string | Date;
}

/**
 * The shape `bun run scripts/diagnostics/run.ts --ingest <file>` expects (mirrors
 * `IngestFinding` in `desktop-app/scripts/diagnostics/run.ts`). Duplicated here on purpose per
 * C3's spec — that script is build tooling outside this project's import graph, so this is a
 * local copy of the contract, not an import.
 */
export interface IngestFinding {
    kind: "bug";
    fingerprint: string;
    area: string;
    title: string;
    body: string;
    error_name?: string;
}

export interface GroupDetail {
    group: ErrorGroup;
    occurrences: ErrorOccurrence[];
}

function toDate(value: string | Date): Date {
    return value instanceof Date ? value : new Date(value);
}

/**
 * Error telemetry domain (plan 23, C3 / D13). Devices batch-report fingerprinted errors; groups
 * are upserted by `fingerprint`, occurrences are tracked per `(groupId, deviceId, appVersion)` and
 * bounded to the newest `MAX_OCCURRENCES_PER_GROUP` rows so a noisy single device can't grow a
 * group's occurrence table without bound.
 */
export class TelemetryService {
    /**
     * Ingests one device's error batch inside a single transaction: for each item, upsert the
     * `error_group` (by fingerprint), upsert its `error_occurrence` row for this device+version,
     * recompute `deviceCount` from distinct occurrence rows, then trim the group's occurrences
     * back down to the newest `MAX_OCCURRENCES_PER_GROUP`.
     *
     * Callers (the device controller) are responsible for checking `telemetryEnabled` before
     * calling this — the service itself has no opinion on that toggle, it only does the counting.
     */
    async ingestErrors(deviceId: string, appVersion: string, batch: IngestBatchItem[]): Promise<void> {
        if (batch.length === 0) return;

        await db.transaction(async (tx) => {
            for (const item of batch) {
                const firstSeen = toDate(item.firstSeen);
                const lastSeen = toDate(item.lastSeen);

                const [group] = await tx
                    .insert(errorGroup)
                    .values({
                        fingerprint: item.fingerprint,
                        code: item.code,
                        source: item.source,
                        message: item.message,
                        firstSeen,
                        lastSeen,
                        totalCount: item.count,
                        deviceCount: 0, // recomputed below from error_occurrence
                        versions: [appVersion],
                    })
                    .onConflictDoUpdate({
                        target: errorGroup.fingerprint,
                        set: {
                            code: item.code,
                            source: item.source,
                            message: item.message,
                            lastSeen: sql`greatest(${errorGroup.lastSeen}, excluded.last_seen)`,
                            firstSeen: sql`least(${errorGroup.firstSeen}, excluded.first_seen)`,
                            totalCount: sql`${errorGroup.totalCount} + ${item.count}`,
                            versions: sql`
                                case
                                    when ${appVersion} = any(${errorGroup.versions})
                                    then ${errorGroup.versions}
                                    else array_append(${errorGroup.versions}, ${appVersion})
                                end
                            `,
                        },
                    })
                    .returning();

                const groupId = group.id;

                await tx
                    .insert(errorOccurrence)
                    .values({
                        groupId,
                        deviceId,
                        appVersion,
                        count: item.count,
                        firstSeen,
                        lastSeen,
                    })
                    .onConflictDoUpdate({
                        target: [errorOccurrence.groupId, errorOccurrence.deviceId, errorOccurrence.appVersion],
                        set: {
                            count: sql`${errorOccurrence.count} + ${item.count}`,
                            lastSeen: sql`greatest(${errorOccurrence.lastSeen}, excluded.last_seen)`,
                        },
                    });

                const [{ deviceCount }] = await tx
                    .select({ deviceCount: sql<number>`count(distinct ${errorOccurrence.deviceId})::int` })
                    .from(errorOccurrence)
                    .where(eq(errorOccurrence.groupId, groupId));

                await tx.update(errorGroup).set({ deviceCount }).where(eq(errorGroup.id, groupId));

                await this.trimOccurrences(tx, groupId);
            }
        });
    }

    /**
     * Deletes the oldest (by lastSeen) occurrence rows beyond the newest `MAX_OCCURRENCES_PER_GROUP`,
     * in one round trip (a correlated subquery + OFFSET), not one DELETE per stale row — this runs on
     * every ingest call against a remote Postgres instance, so round trips matter.
     */
    private async trimOccurrences(tx: Parameters<Parameters<typeof db.transaction>[0]>[0], groupId: string): Promise<void> {
        await tx.execute(sql`
            delete from ${errorOccurrence}
            where id in (
                select id from ${errorOccurrence}
                where group_id = ${groupId}
                order by last_seen desc
                offset ${MAX_OCCURRENCES_PER_GROUP}
            )
        `);
    }

    /** Admin error-groups list, sorted by affected devices (deviceCount desc) by default. */
    async listGroups(params: {
        search?: string;
        versionFilter?: string;
        page?: number;
        limit?: number;
    }): Promise<PaginatedResult<ErrorGroup>> {
        const page = Math.max(1, Number(params.page) || 1);
        const limit = Math.min(100, Math.max(1, Number(params.limit) || 20));
        const offset = (page - 1) * limit;

        const clauses: SQL[] = [];
        if (params.search) {
            clauses.push(sql`(${errorGroup.message} ilike ${"%" + params.search + "%"} or ${errorGroup.code} ilike ${"%" + params.search + "%"})`);
        }
        if (params.versionFilter) {
            clauses.push(sql`${errorGroup.versions} @> ARRAY[${params.versionFilter}]::text[]`);
        }
        const whereClause = clauses.length ? and(...clauses) : undefined;

        const [rows, totalRow] = await Promise.all([
            db
                .select()
                .from(errorGroup)
                .where(whereClause)
                .orderBy(desc(errorGroup.deviceCount))
                .limit(limit)
                .offset(offset),
            db.select({ count: sql<number>`count(*)::int` }).from(errorGroup).where(whereClause),
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

    /** Group row plus its occurrence rows (newest first), for the admin detail view. */
    async getGroupDetail(groupId: string): Promise<GroupDetail | null> {
        const [group] = await db.select().from(errorGroup).where(eq(errorGroup.id, groupId));
        if (!group) return null;

        const occurrences = await db
            .select()
            .from(errorOccurrence)
            .where(eq(errorOccurrence.groupId, groupId))
            .orderBy(desc(errorOccurrence.lastSeen));

        return { group, occurrences };
    }

    /**
     * Maps `error_group` rows to the `IngestFinding` shape for `bun run scripts/diagnostics/run.ts
     * --ingest <file>`. All groups, or just the given ids when provided.
     */
    async exportForIngestLedger(groupIds?: string[]): Promise<IngestFinding[]> {
        const rows =
            groupIds && groupIds.length > 0
                ? await db.select().from(errorGroup).where(inArray(errorGroup.id, groupIds)).orderBy(asc(errorGroup.firstSeen))
                : await db.select().from(errorGroup).orderBy(asc(errorGroup.firstSeen));

        return rows.map((group) => ({
            kind: "bug",
            fingerprint: group.fingerprint,
            area: group.source,
            title: group.message.slice(0, 120),
            body: [
                group.message,
                "",
                `Code: ${group.code}`,
                `Affected devices: ${group.deviceCount}`,
                `Versions: ${group.versions.join(", ")}`,
                `First seen: ${group.firstSeen.toISOString()}`,
                `Last seen: ${group.lastSeen.toISOString()}`,
            ].join("\n"),
            error_name: group.code,
        }));
    }
}

export const telemetryService = new TelemetryService();
export default telemetryService;
