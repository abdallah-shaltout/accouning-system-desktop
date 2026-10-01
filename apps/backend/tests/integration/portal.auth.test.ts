import request from "supertest";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { otp } from "@@otp/otp.schema";
import app from "../../src/app";

const PHONE = "01012345678";
const PASSWORD = "Sup3rSecret!";
const ORG_NAME = "متجر تجريبي";

// The dev console adapter logs the plaintext code (otp.service.ts's deliverOtp fallback);
// tests capture it from console.log since the DB only stores a hash.

describe("portal auth", () => {
    it("rejects signup with a non-Egyptian phone", async () => {
        const res = await request(app)
            .post("/api/portal/auth/signup")
            .send({ phone: "12345", password: PASSWORD, orgName: ORG_NAME });
        expect(res.status).toBe(422);
    });

    it("signs up, verifies OTP, creates the org + owner user, and logs in", async () => {
        const logSpy = vi.spyOn(console, "log").mockImplementation(() => undefined);

        const signup = await request(app)
            .post("/api/portal/auth/signup")
            .send({ phone: PHONE, password: PASSWORD, orgName: ORG_NAME });
        expect(signup.status).toBe(200);

        const otpLog = logSpy.mock.calls.map((c) => String(c[0])).find((line) => line.includes("[DEV OTP]"));
        expect(otpLog).toBeDefined();
        const code = otpLog!.split(":").pop()!.trim();
        logSpy.mockRestore();

        const verify = await request(app).post("/api/portal/auth/verify-otp").send({ phone: PHONE, code });
        expect(verify.status).toBe(201);
        expect(verify.body.data.token).toBeDefined();
        const cookie = verify.headers["set-cookie"];

        const me = await request(app)
            .get("/api/portal/auth/me")
            .set("Authorization", `Bearer ${verify.body.data.token}`);
        expect(me.status).toBe(200);
        expect(me.body.data.role).toBe("owner");

        // A second login with the same credentials works independently of the signup session.
        const login = await request(app).post("/api/portal/auth/login").send({ phone: PHONE, password: PASSWORD });
        expect(login.status).toBe(200);

        // Refresh rotation works the same way as admin.
        const refreshed = await request(app).post("/api/portal/auth/refresh").set("Cookie", cookie);
        expect(refreshed.status).toBe(200);
    });

    it("rejects OTP verification with a wrong code", async () => {
        const logSpy = vi.spyOn(console, "log").mockImplementation(() => undefined);
        await request(app).post("/api/portal/auth/signup").send({ phone: PHONE, password: PASSWORD, orgName: ORG_NAME });
        logSpy.mockRestore();

        const res = await request(app).post("/api/portal/auth/verify-otp").send({ phone: PHONE, code: "000000" });
        expect(res.status).toBe(401);
    });

    it("locks out after 5 failed OTP attempts", async () => {
        const logSpy = vi.spyOn(console, "log").mockImplementation(() => undefined);
        await request(app).post("/api/portal/auth/signup").send({ phone: PHONE, password: PASSWORD, orgName: ORG_NAME });
        logSpy.mockRestore();

        for (let i = 0; i < 5; i++) {
            await request(app).post("/api/portal/auth/verify-otp").send({ phone: PHONE, code: "000000" });
        }
        const rows = await db.select().from(otp).where(eq(otp.phone, PHONE));
        expect(rows[0]?.consumedAt).not.toBeNull();
    });
});
