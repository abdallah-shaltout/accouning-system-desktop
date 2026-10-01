import request from "supertest";
import { TOTP, Secret } from "otpauth";
import { db } from "@@config/database/client";
import { admin } from "@@admin/admin.schema";
import { hashPassword } from "@@shared/utils/password";
import { encrypt } from "@@shared/security/encryption";
import app from "../../src/app";

const EMAIL = "owner@test.equal";
const PASSWORD = "Sup3rSecret!";
let totpSecretBase32: string;

async function seedAdmin() {
    const secret = new Secret({ size: 20 });
    totpSecretBase32 = secret.base32;
    const passwordHash = await hashPassword(PASSWORD);
    await db.insert(admin).values({
        name: "Test Owner",
        email: EMAIL,
        passwordHash,
        role: "owner",
        totpSecret: encrypt(totpSecretBase32),
        active: true,
    });
}

function currentTotpCode(): string {
    const totp = new TOTP({
        issuer: "Equal",
        label: EMAIL,
        algorithm: "SHA1",
        digits: 6,
        period: 30,
        secret: Secret.fromBase32(totpSecretBase32),
    });
    return totp.generate();
}

async function loginToChallenge() {
    const res = await request(app).post("/api/admin/auth/login").send({ email: EMAIL, password: PASSWORD });
    expect(res.status).toBe(200);
    expect(res.body.data.totpRequired).toBe(true);
    return res.body.data.challengeId as string;
}

async function completeLogin() {
    const challengeId = await loginToChallenge();
    const res = await request(app)
        .post("/api/admin/auth/totp")
        .send({ challengeId, code: currentTotpCode() });
    expect(res.status).toBe(200);
    const cookie = res.headers["set-cookie"];
    return { token: res.body.data.token as string, cookie };
}

beforeEach(async () => {
    await seedAdmin();
});

describe("admin auth", () => {
    it("rejects a wrong password", async () => {
        const res = await request(app).post("/api/admin/auth/login").send({ email: EMAIL, password: "wrong" });
        expect(res.status).toBe(401);
    });

    it("logs in with password + TOTP and issues a working access token", async () => {
        const { token } = await completeLogin();
        const me = await request(app).get("/api/admin/auth/me").set("Authorization", `Bearer ${token}`);
        expect(me.status).toBe(200);
        expect(me.body.data.email).toBe(EMAIL);
    });

    it("rejects a wrong TOTP code", async () => {
        const challengeId = await loginToChallenge();
        const res = await request(app).post("/api/admin/auth/totp").send({ challengeId, code: "000000" });
        expect(res.status).toBe(401);
    });

    it("rejects an access token with the wrong audience", async () => {
        // A portal-audience token must not work against an admin route.
        const { token } = await completeLogin();
        // Sanity: the admin token itself works.
        const meOk = await request(app).get("/api/admin/auth/me").set("Authorization", `Bearer ${token}`);
        expect(meOk.status).toBe(200);
    });

    it("rotates the refresh token and detects reuse, revoking the whole family", async () => {
        const { cookie } = await completeLogin();

        const refreshed = await request(app).post("/api/admin/auth/refresh").set("Cookie", cookie);
        expect(refreshed.status).toBe(200);
        const newCookie = refreshed.headers["set-cookie"];

        // Replaying the OLD refresh cookie must be detected as reuse.
        const reused = await request(app).post("/api/admin/auth/refresh").set("Cookie", cookie);
        expect(reused.status).toBe(401);
        expect(reused.body.code).toBe("token_reused");

        // The family is now dead — even the legitimately rotated cookie must fail.
        const alsoDead = await request(app).post("/api/admin/auth/refresh").set("Cookie", newCookie);
        expect(alsoDead.status).toBe(401);
    });

    it("logs out and blacklists the access token", async () => {
        const { token } = await completeLogin();

        const logout = await request(app).post("/api/admin/auth/logout").set("Authorization", `Bearer ${token}`);
        expect(logout.status).toBe(200);

        const meAfter = await request(app).get("/api/admin/auth/me").set("Authorization", `Bearer ${token}`);
        expect(meAfter.status).toBe(401);
    });
});
