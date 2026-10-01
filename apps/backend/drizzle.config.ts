import "dotenv/config";
import { defineConfig } from "drizzle-kit";

if (!process.env.DB_URL) {
    throw new Error("DB_URL is required for drizzle-kit");
}

export default defineConfig({
    schema: "./src/domains/*/*.schema.ts",
    out: "./src/config/database/migrations",
    dialect: "postgresql",
    dbCredentials: {
        url: process.env.DB_URL,
    },
    verbose: true,
    strict: true,
});
