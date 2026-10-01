import { Router } from "express";
import authRoutes from "../services/auth/auth.route";
import planRoutes from "../domains/plan/admin.plan.route";
import organizationRoutes from "../domains/organization/admin.organization.route";
import paymentRoutes from "../domains/payment/admin.payment.route";
import releaseRoutes from "../domains/release/admin.release.route";
import analyticsRoutes from "../domains/analytics/admin.analytics.route";
import telemetryRoutes from "../domains/telemetry/admin.telemetry.route";
import diagnosticsRoutes from "../domains/diagnostics/admin.diagnostics.route";
import feedbackRoutes from "../domains/feedback/admin.feedback.route";

const router = Router();

router.use("/auth", authRoutes);
router.use("/plans", planRoutes);
router.use("/organizations", organizationRoutes);
router.use("/payments", paymentRoutes);
router.use("/releases", releaseRoutes);
router.use("/analytics", analyticsRoutes);
router.use("/telemetry", telemetryRoutes);
router.use("/diagnostics", diagnosticsRoutes);
router.use("/feedback", feedbackRoutes);

export default router;
