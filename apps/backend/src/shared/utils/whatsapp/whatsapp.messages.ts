import type { AxiosInstance } from "axios";
import type { GowaPluginResult, GowaSendTextOptions } from "./whatsapp.types";
import { normalizePhone, wrapApiError } from "./whatsapp.client";

function isSuccessStatus(status: number): boolean {
    return status >= 200 && status < 300;
}

export async function sendTextMessage(
    client: AxiosInstance,
    options: GowaSendTextOptions,
): Promise<GowaPluginResult<unknown>> {
    try {
        const { to, message } = options;
        const response = await client.post("/send/message", {
            phone: normalizePhone(to),
            message,
        });

        if (!isSuccessStatus(response.status)) {
            return {
                success: false,
                error: {
                    message: "Failed to send WhatsApp message",
                    statusCode: response.status,
                    details: response.data,
                },
            };
        }

        return { success: true, data: response.data };
    } catch (error: unknown) {
        return {
            success: false,
            error: wrapApiError(error, "Failed to send WhatsApp message"),
        };
    }
}

export async function markMessageAsRead(
    client: AxiosInstance,
    messageId: string,
    phone: string,
): Promise<GowaPluginResult<unknown>> {
    try {
        const response = await client.post(`/message/${encodeURIComponent(messageId)}/read`, {
            phone: normalizePhone(phone),
        });

        if (!isSuccessStatus(response.status)) {
            return {
                success: false,
                error: {
                    message: "Failed to mark message as read",
                    statusCode: response.status,
                    details: response.data,
                },
            };
        }

        return { success: true, data: response.data };
    } catch (error: unknown) {
        return {
            success: false,
            error: wrapApiError(error, "Failed to mark message as read"),
        };
    }
}

export async function getStatus(client: AxiosInstance): Promise<GowaPluginResult<unknown>> {
    try {
        const response = await client.get("/app/status");

        if (!isSuccessStatus(response.status)) {
            return {
                success: false,
                error: {
                    message: "Failed to fetch GOWA status",
                    statusCode: response.status,
                    details: response.data,
                },
            };
        }

        return { success: true, data: response.data };
    } catch (error: unknown) {
        return {
            success: false,
            error: wrapApiError(error, "Failed to fetch GOWA status"),
        };
    }
}
