import { Router } from "express";
import { authRateLimit } from "@@shared/middleware/rateLimit/authRateLimit";
import { AdminRequiredAuth } from "@@shared/middleware/auth/admin.protacted";
import { adminAuthController } from "./auth.controller";
import { loginValidation, totpValidation } from "./auth.validation";

const router = Router();

// Public — no token yet.
router.post("/login", authRateLimit, ...loginValidation, adminAuthController.login);
router.post("/totp", authRateLimit, ...totpValidation, adminAuthController.totp);
router.post("/refresh", adminAuthController.refresh);

// Session-required.
router.post("/logout", AdminRequiredAuth, adminAuthController.logout);
router.post("/logout-all", AdminRequiredAuth, adminAuthController.logoutAll);
router.get("/me", AdminRequiredAuth, adminAuthController.me);

export default router;
