// Aggregates every domain's Drizzle schema so drizzle-orm's query builder and drizzle-kit see one graph.
// Each domain owns its own `<d>.schema.ts`; this file only re-exports. Never hand-write tables here.

export * from "../../domains/admin/admin.schema";
export * from "../../domains/adminActivity/adminActivity.schema";
export * from "../../domains/organization/organization.schema";
export * from "../../domains/user/user.schema";
export * from "../../domains/otp/otp.schema";
export * from "../../domains/plan/plan.schema";
export * from "../../domains/subscription/subscription.schema";
export * from "../../domains/payment/payment.schema";
export * from "../../domains/device/device.schema";
export * from "../../domains/activation/activation.schema";
export * from "../../domains/license/license.schema";
export * from "../../domains/credit/credit.schema";
export * from "../../domains/release/release.schema";
export * from "../../domains/telemetry/telemetry.schema";
export * from "../../domains/diagnostics/diagnostics.schema";
export * from "../../domains/feedback/feedback.schema";
