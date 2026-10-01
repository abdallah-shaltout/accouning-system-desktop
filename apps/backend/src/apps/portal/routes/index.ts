import { Router } from "express";
import authRoutes from "@/portal/services/auth/auth.route";
import organizationRoutes from "@/portal/domains/organization/portal.organization.route";
import userRoutes from "@/portal/domains/user/portal.user.route";
import planRoutes from "@/portal/domains/plan/portal.plan.route";
import deviceRoutes from "@/portal/domains/device/portal.device.route";
import activationRoutes from "@/portal/domains/activation/portal.activation.route";
import creditRoutes from "@/portal/domains/credit/portal.credit.route";
import subscriptionRoutes from "@/portal/domains/subscription/portal.subscription.route";
import paymentRoutes from "@/portal/domains/payment/portal.payment.route";
import invoiceRoutes from "@/portal/domains/payment/portal.invoice.route";
import paymentInstructionsRoutes from "@/portal/domains/payment/portal.paymentInstructions.route";

const router = Router();

router.use("/auth", authRoutes);
router.use("/organization", organizationRoutes);
router.use("/users", userRoutes);
// Public — no auth. See portal.plan.route.ts.
router.use("/plans", planRoutes);
router.use("/devices", deviceRoutes);
router.use("/activation", activationRoutes);
router.use("/credits", creditRoutes);
router.use("/subscription", subscriptionRoutes);
router.use("/payments", paymentRoutes);
router.use("/invoices", invoiceRoutes);
router.use("/payment-instructions", paymentInstructionsRoutes);

export default router;
