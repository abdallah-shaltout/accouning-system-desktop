import "dotenv/config";
import { migrate } from "drizzle-orm/node-postgres/migrator";
import { db, pool } from "./client";

async function main() {
    await migrate(db, { migrationsFolder: "./src/config/database/migrations" });
    await pool.end();
    console.log("Migrations applied.");
}

main().catch((err) => {
    console.error("Migration failed:", err);
    process.exit(1);
});
