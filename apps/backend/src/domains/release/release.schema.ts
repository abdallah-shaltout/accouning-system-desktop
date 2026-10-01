import { pgTable, uuid, text, timestamp, boolean, integer, pgEnum, uniqueIndex } from "drizzle-orm/pg-core";

export const releaseChannelEnum = pgEnum("release_channel", ["stable", "beta"]);

export const release = pgTable(
    "release",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        version: text("version").notNull(), // semver, e.g. "1.4.0"
        channel: releaseChannelEnum("channel").notNull().default("stable"),
        notes: text("notes"), // Arabic release notes
        fileKey: text("file_key").notNull(), // R2_BUCKET_RELEASES object key (NSIS installer)
        url: text("url").notNull(), // public R2 URL
        signature: text("signature").notNull(), // Tauri updater .sig contents
        rolloutPercent: integer("rollout_percent").notNull().default(0),
        isMandatory: boolean("is_mandatory").notNull().default(false),
        publishedAt: timestamp("published_at", { withTimezone: true }),
        pausedAt: timestamp("paused_at", { withTimezone: true }),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [uniqueIndex("release_channel_version_unique").on(table.channel, table.version)],
);

export type Release = typeof release.$inferSelect;
export type NewRelease = typeof release.$inferInsert;
