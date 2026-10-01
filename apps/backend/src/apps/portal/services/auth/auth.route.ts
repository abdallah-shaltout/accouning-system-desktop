import { Router } from "express";
import { PortalRequiredAuth } from "@@shared/middleware/auth/portal.protacted";
import { authRateLimit, otpSendRateLimit } from "@@shared/middleware/rateLimit/authRateLimit";
import { authController } from "./auth.controller";
import { authValidation } from "./auth.validation";

const router = Router();

// Public, rate-limited account endpoints.
router.post("/signup", otpSendRateLimit, authValidation.signup, authController.signup);
router.post("/verify-otp", authValidation.verifyOtp, authController.verifyOtp);
router.post("/login", authRateLimit, authValidation.login, authController.login);
router.post("/forgot", otpSendRateLimit, authValidation.forgot, authController.forgot);
router.post("/reset", authValidation.reset, authController.reset);

// Reads the refresh cookie itself; no access-token auth required.
router.post("/refresh", authController.refresh);

// Session management — requires a valid portal access token.
router.post("/logout", PortalRequiredAuth, authController.logout);
router.post("/logout-all", PortalRequiredAuth, authController.logoutAll);
router.get("/me", PortalRequiredAuth, authController.me);

export default router;
