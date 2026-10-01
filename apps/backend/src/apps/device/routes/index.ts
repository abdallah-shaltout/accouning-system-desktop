import { Router } from "express";
import deviceRoutes from "@/device/domains/device/device.route";
import creditRoutes from "@/device/domains/credit/device.credit.route";
import releaseRoutes from "@/device/domains/release/device.release.route";
import telemetryRoutes from "@/device/domains/telemetry/device.telemetry.route";
import diagnosticsRoutes from "@/device/domains/diagnostics/device.diagnostics.route";
import feedbackRoutes from "@/device/domains/feedback/device.feedback.route";

// Phase B adds: register, activate, heartbeat, credits (below). Later phases add: releases,
// telemetry, diagnostics, feedback (all landed).
const router = Router();

router.use("/", deviceRoutes);
router.use("/credits", creditRoutes);
router.use("/releases", releaseRoutes);
router.use("/telemetry", telemetryRoutes);
router.use("/diagnostics", diagnosticsRoutes);
router.use("/feedback", feedbackRoutes);

export default router;
