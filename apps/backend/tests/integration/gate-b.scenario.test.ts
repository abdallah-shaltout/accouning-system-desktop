import crypto from "node:crypto";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { organization } from "@@organization/organization.schema";
import { user } from "@@user/user.schema";
import { plan, planVersion } from "@@plan/plan.schema";
import { device } from "@@device/device.schema";
import { subscription } from "@@subscription/subscription.schema";
import { subscriptionService } from "@@subscription/subscription.service";
import { paymentService } from "@@payment/payment.service";
import { deviceService } from "@@device/device.service";
import { activationService } from "@@activation/activation.service";
import { licenseService, type LicensePayload } from "@@license/license.service";
import { runSubscriptionLifecycleTick } from "@@shared/cron/subscriptionLifecycle";
import { verifyToken } from "@@shared/security/licenseSigner";
import { generateUUID } from "@@shared/utils/uuid";
import { seedPlans } from "../../src/config/seeders/01.seedPlans";

beforeAll(async () => {
    // tests/setup/setup.ts truncates every table afterEach — the dev-only plan seed doesn't reach
    // the test DB, so this scenario seeds its own catalog the same idempotent way `bun run seed` does.
    await seedPlans();
});

/**
 * Gate B's end-to-end scenario (phase-b-server-billing-licensing.md): signup → checkout Pro →
 * manual payment → admin approve → a device activates by code → the license has Pro entitlements
 * → the cron expires it → heartbeat returns the Free policy.
 *
 * Runs directly against the service layer (not HTTP) to isolate this from R2/multer concerns —
 * the HTTP-level payment upload path is covered by payment.service.test.ts. This test's job is to
 * prove the cross-domain wiring itself: subscription → payment → license → cron → license, using
 * the exact `subscriptionService`/`paymentService`/`activationService`/`licenseService` singletons
 * every route handler calls.
 */
describe("Gate B scenario: signup -> Pro -> device activation -> lapse -> Free", () => {
    it("carries an org from checkout through an active Pro license to expiry and back to Free", async () => {
        // 1. "Signup": an org + owner user (bypassing the OTP flow, already covered by portal.auth.test.ts).
        const [org] = await db
            .insert(organization)
            .values({ name: "متجر الاختبار", phone: "01099999999" })
            .returning();
        const [owner] = await db
            .insert(user)
            .values({
                orgId: org.id,
                name: "Owner",
                phone: "01099999999",
                passwordHash: "not-used-in-this-test",
                role: "owner",
                phoneVerifiedAt: new Date(),
            })
            .returning();

        // 2. Checkout Pro (the seeder already published Pro v1 — read it back).
        const [proPlan] = await db.select().from(plan).where(eq(plan.key, "pro"));
        expect(proPlan).toBeDefined();
        const [proVersion] = await db
            .select()
            .from(planVersion)
            .where(eq(planVersion.planId, proPlan.id));
        expect(proVersion?.status).toBe("published");

        const { subscription: sub, invoice } = await subscriptionService.startCheckout(
            org.id,
            proVersion.id,
            "month",
            owner.id,
        );
        expect(sub.status).toBe("pending_payment");
        expect(invoice.status).toBe("open");
        expect(invoice.amount).toBe(proVersion.priceMonthly);

        // 3. Manual payment submission (no receipt file — R2 isn't configured in this env, and the
        // service already supports a fileless submission per its own type signature).
        const submitted = await paymentService.submitManualPayment({
            orgId: org.id,
            invoiceId: invoice.id,
            method: "instapay",
            reference: "TESTREF123",
            receiptFile: null,
        });
        expect(submitted.status).toBe("pending");

        // 4. Admin approves -> emits payment.approved -> subscription transitions to active.
        const approved = await paymentService.approvePayment(submitted.id, generateUUID());
        expect(approved.status).toBe("approved");

        // The bus handler runs asynchronously after commit; wait for the subscription to flip.
        let activeSub = await subscriptionService.readDocumentById(sub.id);
        for (let i = 0; i < 20 && activeSub?.status !== "active"; i++) {
            await new Promise((r) => setTimeout(r, 50));
            activeSub = await subscriptionService.readDocumentById(sub.id);
        }
        expect(activeSub?.status).toBe("active");
        expect(activeSub?.currentPeriodEnd.getTime()).toBeGreaterThan(Date.now());

        // 5. A device activates by code and receives a Pro license.
        const [mainDevice] = await db
            .insert(device)
            .values({ terminalId: `terminal-${generateUUID()}`, role: "main" })
            .returning();

        const verifier = crypto.randomBytes(32).toString("hex");
        const challenge = crypto.createHash("sha256").update(verifier).digest("hex");

        const approval = await activationService.approve({
            challenge,
            state: "test-state",
            terminalId: mainDevice.terminalId,
            deviceName: "Main PC",
            orgId: org.id,
            userId: owner.id,
        });

        const activation = await activationService.activate(mainDevice.id, "main", {
            code: approval.code,
            verifier,
        });
        expect(activation.org.id).toBe(org.id);

        const publicKeyB64 = await getPublicKeyForActiveKid();
        const decodedPayload = (await verifyToken(activation.license, publicKeyB64)) as unknown as LicensePayload;
        expect(decodedPayload.plan).toBe("pro");
        expect(decodedPayload.ent.limits.maxProducts).toBeNull(); // Pro: unlimited products
        expect(decodedPayload.org).toBe(org.id);

        // 6. The cron expires the subscription once its period has passed.
        await db
            .update(subscription)
            .set({ status: "active", currentPeriodEnd: new Date(Date.now() - 1000) })
            .where(eq(subscription.id, sub.id));
        await runSubscriptionLifecycleTick(); // active -> past_due -> grace
        await runSubscriptionLifecycleTick(); // (idempotent no-op on the same tick's fresh grace window)

        const afterFirstTick = await subscriptionService.readDocumentById(sub.id);
        expect(afterFirstTick?.status).toBe("grace");

        await db
            .update(subscription)
            .set({ graceUntil: new Date(Date.now() - 1000) })
            .where(eq(subscription.id, sub.id));
        await runSubscriptionLifecycleTick(); // grace -> expired

        const expiredSub = await subscriptionService.readDocumentById(sub.id);
        expect(expiredSub?.status).toBe("expired");

        // 7. A fresh license issuance ("heartbeat") for the same device now returns the Free policy.
        const reissuedToken = await licenseService.issueFor(mainDevice.id);
        const reissuedPayload = (await verifyToken(reissuedToken, publicKeyB64)) as unknown as LicensePayload;
        expect(reissuedPayload.plan).toBe("free");
        expect(reissuedPayload.ent.limits.maxProducts).toBe(100);

        // getEffectiveEntitlements itself also reports Free now, confirming the cross-domain seam.
        const effective = await subscriptionService.getEffectiveEntitlements(org.id);
        expect(effective.planKey).toBe("free");
    });
});

async function getPublicKeyForActiveKid(): Promise<string> {
    const { generateKeyPair } = await import("@@shared/security/licenseSigner");
    // The active kid's public key isn't exposed by validateEnv (private-key-only surface on the
    // server); derive it the same way the desktop would trust it — from LICENSE_SIGNING_KEY_<kid>
    // via @noble/ed25519, mirroring keys:generate's own derivation.
    const kid = process.env.LICENSE_ACTIVE_KID!;
    const privateKeyB64 = process.env[`LICENSE_SIGNING_KEY_${kid}`]!;
    const ed25519 = await import("@noble/ed25519");
    const privateKey = new Uint8Array(Buffer.from(privateKeyB64, "base64"));
    const publicKey = await ed25519.getPublicKeyAsync(privateKey);
    void generateKeyPair; // referenced for readability only
    return Buffer.from(publicKey).toString("base64");
}
