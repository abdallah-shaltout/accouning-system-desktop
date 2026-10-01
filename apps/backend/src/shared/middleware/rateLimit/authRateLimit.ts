import rateLimit from "express-rate-limit";

// Vitest integration tests exercise the same endpoints dozens of times per run from one IP;
// real rate limiting would make the suite flaky rather than testing anything meaningful.
// Only the test runner sets this (vitest.config.ts) — DEV and PROD both enforce the real limits.
const skip = () => process.env.TEST_DISABLE_RATE_LIMIT === "true";

/** 5 attempts / 15 min per IP, for admin/portal login and OTP-gated auth endpoints. */
export const authRateLimit = rateLimit({
    windowMs: 15 * 60 * 1000,
    limit: 5,
    standardHeaders: true,
    legacyHeaders: false,
    skip,
    message: { status: "fail", success: false, message: "محاولات كثيرة جدًا، حاول بعد قليل", code: "rate_limited" },
});

/** 3 / hour per IP, for OTP send endpoints (also rate-limited per-phone in the otp domain). */
export const otpSendRateLimit = rateLimit({
    windowMs: 60 * 60 * 1000,
    limit: 3,
    standardHeaders: true,
    legacyHeaders: false,
    skip,
    message: { status: "fail", success: false, message: "محاولات كثيرة جدًا، حاول بعد ساعة", code: "rate_limited" },
});

/** 10 / hour per IP, for device registration. */
export const deviceRegisterRateLimit = rateLimit({
    windowMs: 60 * 60 * 1000,
    limit: 10,
    standardHeaders: true,
    legacyHeaders: false,
    skip,
    message: { status: "fail", success: false, message: "محاولات كثيرة جدًا، حاول بعد ساعة", code: "rate_limited" },
});

/** 10 / hour per device or org, for activation endpoints. */
export const activationRateLimit = rateLimit({
    windowMs: 60 * 60 * 1000,
    limit: 10,
    standardHeaders: true,
    legacyHeaders: false,
    skip,
    message: { status: "fail", success: false, message: "محاولات كثيرة جدًا، حاول بعد ساعة", code: "rate_limited" },
});

/** 60 / min per device, for the remaining device endpoints. */
export const deviceRateLimit = rateLimit({
    windowMs: 60 * 1000,
    limit: 60,
    standardHeaders: true,
    legacyHeaders: false,
    skip,
    message: { status: "fail", success: false, message: "طلبات كثيرة جدًا", code: "rate_limited" },
});
