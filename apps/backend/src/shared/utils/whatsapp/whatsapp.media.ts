import type { AxiosInstance } from "axios";
import type { GowaPluginResult, GowaSendImageOptions } from "./whatsapp.types";
import { normalizePhone, wrapApiError } from "./whatsapp.client";

function isSuccessStatus(status: number): boolean {
    return status >= 200 && status < 300;
}

/**
 * Sends an image via GOWA's `/send/image` (multipart form) endpoint. Mirrors the reference GOWA
 * service's `sendImage` (references/.../gowa.service.ts) — either an uploaded buffer or a remote
 * `imageUrl` is required.
 */
export async function sendImage(
    client: AxiosInstance,
    options: GowaSendImageOptions,
): Promise<GowaPluginResult<unknown>> {
    try {
        const { to, caption, imageUrl, imageBuffer, imageFilename, imageMimetype, viewOnce, compress } =
            options;

        if (!imageBuffer && !imageUrl) {
            return {
                success: false,
                error: { message: "imageBuffer or imageUrl is required", statusCode: 400 },
            };
        }

        const FormDataCtor = (await import("form-data")).default;
        const form = new FormDataCtor();
        form.append("phone", normalizePhone(to));
        if (caption !== undefined) form.append("caption", caption);
        if (viewOnce !== undefined) form.append("view_once", String(viewOnce));
        if (compress !== undefined) form.append("compress", String(compress));

        if (imageBuffer) {
            form.append("image", imageBuffer, {
                filename: imageFilename ?? "image.jpg",
                contentType: imageMimetype ?? "image/jpeg",
            });
        } else if (imageUrl) {
            form.append("image_url", imageUrl);
        }

        const response = await client.post("/send/image", form, {
            headers: form.getHeaders(),
        });

        if (!isSuccessStatus(response.status)) {
            return {
                success: false,
                error: {
                    message: "Failed to send WhatsApp image",
                    statusCode: response.status,
                    details: response.data,
                },
            };
        }

        return { success: true, data: response.data };
    } catch (error: unknown) {
        return {
            success: false,
            error: wrapApiError(error, "Failed to send WhatsApp image"),
        };
    }
}
