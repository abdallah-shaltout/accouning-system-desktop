import type { AxiosInstance } from "axios";
import { createWhatsAppClient } from "./whatsapp.client";
import type { GowaPluginConfig, GowaSendImageOptions, GowaSendTextOptions } from "./whatsapp.types";
import * as messages from "./whatsapp.messages";
import * as media from "./whatsapp.media";
import { ApiError } from "@@shared/middleware/error/apiError";

export class WhatsAppGowaPlugin {
    private client: AxiosInstance | null = null;
    private config: GowaPluginConfig | null = null;

    private ensureClient(): { client: AxiosInstance; config: GowaPluginConfig } {
        if (!this.client || !this.config) {
            const { client, config } = createWhatsAppClient();
            this.client = client;
            this.config = config;
        }
        if (!this.client || !this.config) {
            throw new ApiError({
                message: "WhatsApp (GOWA) client is not initialized",
                statusCode: 500,
            });
        }
        return { client: this.client, config: this.config };
    }

    public async sendTextMessage(options: GowaSendTextOptions) {
        const { client } = this.ensureClient();
        return messages.sendTextMessage(client, options);
    }

    public async sendImage(options: GowaSendImageOptions) {
        const { client } = this.ensureClient();
        return media.sendImage(client, options);
    }

    public async markMessageAsRead(messageId: string, phone: string) {
        const { client } = this.ensureClient();
        return messages.markMessageAsRead(client, messageId, phone);
    }

    public async getStatus() {
        const { client } = this.ensureClient();
        return messages.getStatus(client);
    }
}
