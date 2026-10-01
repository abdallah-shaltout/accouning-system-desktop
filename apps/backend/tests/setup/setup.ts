import "dotenv/config";
import { sql } from "drizzle-orm";
import { db } from "@@config/database/client";
import { redisClient } from "@@config/database/redisConfig";

// Vitest (pool: "forks", isolate: false) can load this setup module once per worker but run
// multiple test files' modules against the same process — so the very first import must capture
// the *original* DB_URL before anything mutates it, or a later file's guard would compare
// TEST_DB_URL against an already-swapped DB_URL and always look equal.
const ORIGINAL_DB_URL = process.env.EQUAL_ORIGINAL_DB_URL ?? process.env.DB_URL;
process.env.EQUAL_ORIGINAL_DB_URL = ORIGINAL_DB_URL;

function assertTestDatabase(): void {
    const testUrl = process.env.TEST_DB_URL;

    if (!testUrl) {
        throw new Error("TEST_DB_URL is required to run tests (refusing to run against DB_URL).");
    }

    const testDbName = new URL(testUrl).pathname.replace("/", "");
    if (!testDbName.endsWith("_test")) {
        throw new Error(`TEST_DB_URL's database ("${testDbName}") must end in _test.`);
    }
    if (testUrl === ORIGINAL_DB_URL) {
        throw new Error("TEST_DB_URL must differ from DB_URL.");
    }

    // Point the pool used by `db` at the test database for the duration of the test run.
    process.env.DB_URL = testUrl;
}

assertTestDatabase();

beforeAll(async () => {
    if (process.env.TEST_REDIS_URI) {
        process.env.REDIS_URI = process.env.TEST_REDIS_URI;
    }
    await redisClient.connect().catch(() => undefined);
});

afterEach(async () => {
    // Truncate every table except drizzle's own migration bookkeeping, cascading FKs.
    const tables = await db.execute(sql`
        select tablename from pg_tables
        where schemaname = 'public' and tablename not like '__drizzle%'
    `);
    const names = (tables as any).rows.map((r: any) => `"${r.tablename}"`).join(", ");
    if (names) {
        await db.execute(sql.raw(`truncate table ${names} restart identity cascade`));
    }
    await redisClient.flushdb().catch(() => undefined);
});

// No pool.end()/redisClient.disconnect() here: with vitest's `isolate: false`, every test file in
// this worker shares the same `db`/`redisClient` module instance, so closing them in this file's
// afterAll would break every other test file still running in the same worker. The process exits
// when the vitest run finishes; the OS reclaims the sockets then.
