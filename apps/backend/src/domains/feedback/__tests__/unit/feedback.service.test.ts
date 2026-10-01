import { describe, expect, it } from "vitest";
import { db } from "@@config/database/client";
import { organization } from "@@organization/organization.schema";
import { device } from "@@device/device.schema";
import { feedbackService } from "../../feedback.service";

async function createOrg(name: string) {
    const [row] = await db.insert(organization).values({ name, phone: "01000000000" }).returning();
    return row;
}

async function createDevice(terminalId: string, orgId: string | null) {
    const [row] = await db.insert(device).values({ terminalId, orgId, role: "main" }).returning();
    return row;
}

describe("feedbackService.submit", () => {
    it("accepts text-only feedback even when R2 isn't configured", async () => {
        const org = await createOrg("منشأة ملاحظات 1");
        const dev = await createDevice("fb-term-1", org.id);

        const created = await feedbackService.submit({
            deviceId: dev.id,
            orgId: org.id,
            message: "التطبيق بطيء عند فتح الفواتير",
        });

        expect(created.status).toBe("new");
        expect(created.screenshotKey).toBeNull();
        expect(created.bundleKey).toBeNull();
    });

    it("throws storage_not_configured when a screenshot is attached but R2 isn't configured", async () => {
        const org = await createOrg("منشأة ملاحظات 2");
        const dev = await createDevice("fb-term-2", org.id);

        await expect(
            feedbackService.submit({
                deviceId: dev.id,
                orgId: org.id,
                message: "مرفق لقطة شاشة",
                screenshotBuffer: Buffer.from("png-bytes"),
            }),
        ).rejects.toMatchObject({ statusCode: 503, code: "storage_not_configured" });
    });

    it("updateStatus transitions the feedback row", async () => {
        const org = await createOrg("منشأة ملاحظات 3");
        const dev = await createDevice("fb-term-3", org.id);

        const created = await feedbackService.submit({
            deviceId: dev.id,
            orgId: org.id,
            message: "اقتراح تحسين",
        });

        const updated = await feedbackService.updateStatus(created.id, "in_progress");
        expect(updated.status).toBe("in_progress");
    });
});
