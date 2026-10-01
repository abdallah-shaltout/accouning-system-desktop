import type { Transporter } from "nodemailer";
import type {
    MailerPluginConfig,
    MailerPluginResult,
    SendOtpOptions,
    SendForgotPasswordOptions,
    SendRawOptions,
    SentMessageInfo,
} from "./nodemailer.types";
import { renderOtpTemplate, renderForgotPasswordTemplate } from "./nodemailer.templates";
import { wrapMailerError } from "./nodemailer.client";

export async function sendOtpEmail(
    transporter: Transporter,
    config: MailerPluginConfig,
    options: SendOtpOptions,
): Promise<MailerPluginResult<SentMessageInfo>> {
    try {
        const html = renderOtpTemplate({
            recipientName: options.recipientName,
            otp: options.otp,
            expiresInMinutes: options.expiresInMinutes,
        });

        const info = await transporter.sendMail({
            from: `"${config.fromName}" <${config.fromAddress}>`,
            to: options.to,
            subject: "رمز التحقق الخاص بك",
            html,
        });

        return {
            success: true,
            data: {
                messageId: info.messageId,
                accepted: info.accepted as string[],
                rejected: info.rejected as string[],
            },
            messageId: info.messageId,
        };
    } catch (error: unknown) {
        return {
            success: false,
            error: wrapMailerError(error, "Failed to send OTP email"),
        };
    }
}

export async function sendForgotPasswordEmail(
    transporter: Transporter,
    config: MailerPluginConfig,
    options: SendForgotPasswordOptions,
): Promise<MailerPluginResult<SentMessageInfo>> {
    try {
        const html = renderForgotPasswordTemplate({
            recipientName: options.recipientName,
            resetLink: options.resetLink,
            expiresInMinutes: options.expiresInMinutes,
        });

        const info = await transporter.sendMail({
            from: `"${config.fromName}" <${config.fromAddress}>`,
            to: options.to,
            subject: "إعادة تعيين كلمة المرور",
            html,
        });

        return {
            success: true,
            data: {
                messageId: info.messageId,
                accepted: info.accepted as string[],
                rejected: info.rejected as string[],
            },
            messageId: info.messageId,
        };
    } catch (error: unknown) {
        return {
            success: false,
            error: wrapMailerError(error, "Failed to send forgot-password email"),
        };
    }
}

export async function sendRawEmail(
    transporter: Transporter,
    config: MailerPluginConfig,
    options: SendRawOptions,
): Promise<MailerPluginResult<SentMessageInfo>> {
    try {
        const info = await transporter.sendMail({
            from: `"${config.fromName}" <${config.fromAddress}>`,
            to: Array.isArray(options.to) ? options.to.join(", ") : options.to,
            subject: options.subject,
            html: options.html,
            text: options.text,
        });

        return {
            success: true,
            data: {
                messageId: info.messageId,
                accepted: info.accepted as string[],
                rejected: info.rejected as string[],
            },
            messageId: info.messageId,
        };
    } catch (error: unknown) {
        return {
            success: false,
            error: wrapMailerError(error, "Failed to send email"),
        };
    }
}
