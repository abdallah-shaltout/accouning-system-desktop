import type { Transporter } from "nodemailer";
import { createMailerClient } from "./nodemailer.client";
import type {
    MailerPluginConfig,
    MailerPluginResult,
    SendOtpOptions,
    SendForgotPasswordOptions,
    SendRawOptions,
    SentMessageInfo,
} from "./nodemailer.types";
import * as messages from "./nodemailer.messages";
import { ApiError } from "@@shared/middleware/error/apiError";

export class NodeMailerPlugin {
    private transporter: Transporter | null = null;
    private config: MailerPluginConfig | null = null;

    private ensureTransporter(): { transporter: Transporter; config: MailerPluginConfig } {
        if (!this.transporter || !this.config) {
            const { transporter, config } = createMailerClient();
            this.transporter = transporter;
            this.config = config;
        }
        if (!this.transporter || !this.config) {
            throw new ApiError({
                message: "Mailer is not initialized",
                statusCode: 500,
            });
        }
        return { transporter: this.transporter, config: this.config };
    }

    public async sendOtp(options: SendOtpOptions): Promise<MailerPluginResult<SentMessageInfo>> {
        const { transporter, config } = this.ensureTransporter();
        return messages.sendOtpEmail(transporter, config, options);
    }

    public async sendForgotPassword(
        options: SendForgotPasswordOptions,
    ): Promise<MailerPluginResult<SentMessageInfo>> {
        const { transporter, config } = this.ensureTransporter();
        return messages.sendForgotPasswordEmail(transporter, config, options);
    }

    public async sendRaw(options: SendRawOptions): Promise<MailerPluginResult<SentMessageInfo>> {
        const { transporter, config } = this.ensureTransporter();
        return messages.sendRawEmail(transporter, config, options);
    }

    public async verifyConnection(): Promise<boolean> {
        try {
            const { transporter } = this.ensureTransporter();
            await transporter.verify();
            return true;
        } catch {
            return false;
        }
    }
}
