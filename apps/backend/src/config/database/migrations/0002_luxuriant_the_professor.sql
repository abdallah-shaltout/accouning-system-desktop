CREATE TYPE "public"."diagnostics_request_status" AS ENUM('pending', 'uploaded', 'declined', 'expired');--> statement-breakpoint
CREATE TYPE "public"."feedback_status" AS ENUM('new', 'in_progress', 'done');--> statement-breakpoint
CREATE TYPE "public"."release_channel" AS ENUM('stable', 'beta');--> statement-breakpoint
CREATE TYPE "public"."error_group_status" AS ENUM('open', 'ignored', 'fixed');--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "diagnostics_request" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"device_id" uuid NOT NULL,
	"org_id" uuid,
	"requested_by" uuid NOT NULL,
	"status" "diagnostics_request_status" DEFAULT 'pending' NOT NULL,
	"file_key" text,
	"size_bytes" bigint,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"fulfilled_at" timestamp with time zone
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "feedback" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"device_id" uuid NOT NULL,
	"org_id" uuid,
	"message" text NOT NULL,
	"screenshot_key" text,
	"bundle_key" text,
	"status" "feedback_status" DEFAULT 'new' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "release" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"version" text NOT NULL,
	"channel" "release_channel" DEFAULT 'stable' NOT NULL,
	"notes" text,
	"file_key" text NOT NULL,
	"url" text NOT NULL,
	"signature" text NOT NULL,
	"rollout_percent" integer DEFAULT 0 NOT NULL,
	"is_mandatory" boolean DEFAULT false NOT NULL,
	"published_at" timestamp with time zone,
	"paused_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "error_group" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"fingerprint" text NOT NULL,
	"code" text NOT NULL,
	"source" text NOT NULL,
	"message" text NOT NULL,
	"first_seen" timestamp with time zone DEFAULT now() NOT NULL,
	"last_seen" timestamp with time zone DEFAULT now() NOT NULL,
	"total_count" integer DEFAULT 0 NOT NULL,
	"device_count" integer DEFAULT 0 NOT NULL,
	"versions" text[] DEFAULT '{}' NOT NULL,
	"status" "error_group_status" DEFAULT 'open' NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "error_occurrence" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"group_id" uuid NOT NULL,
	"device_id" uuid NOT NULL,
	"app_version" text NOT NULL,
	"count" integer DEFAULT 1 NOT NULL,
	"first_seen" timestamp with time zone DEFAULT now() NOT NULL,
	"last_seen" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "diagnostics_request" ADD CONSTRAINT "diagnostics_request_device_id_device_id_fk" FOREIGN KEY ("device_id") REFERENCES "public"."device"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "diagnostics_request" ADD CONSTRAINT "diagnostics_request_org_id_organization_id_fk" FOREIGN KEY ("org_id") REFERENCES "public"."organization"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "feedback" ADD CONSTRAINT "feedback_device_id_device_id_fk" FOREIGN KEY ("device_id") REFERENCES "public"."device"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "feedback" ADD CONSTRAINT "feedback_org_id_organization_id_fk" FOREIGN KEY ("org_id") REFERENCES "public"."organization"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "error_occurrence" ADD CONSTRAINT "error_occurrence_group_id_error_group_id_fk" FOREIGN KEY ("group_id") REFERENCES "public"."error_group"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "error_occurrence" ADD CONSTRAINT "error_occurrence_device_id_device_id_fk" FOREIGN KEY ("device_id") REFERENCES "public"."device"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "diagnostics_request_device_status_idx" ON "diagnostics_request" USING btree ("device_id","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "feedback_status_idx" ON "feedback" USING btree ("status");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "release_channel_version_unique" ON "release" USING btree ("channel","version");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "error_group_fingerprint_unique" ON "error_group" USING btree ("fingerprint");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "error_occurrence_group_device_version_unique" ON "error_occurrence" USING btree ("group_id","device_id","app_version");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "error_occurrence_group_idx" ON "error_occurrence" USING btree ("group_id");