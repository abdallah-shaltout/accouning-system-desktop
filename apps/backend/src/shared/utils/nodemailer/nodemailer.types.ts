export interface MailerPluginConfig {
    host: string;
    port: number;
    secure: boolean;
    user: string;
    pass: string;
    fromName: string;
    fromAddress: string;
}

export interface MailerPluginSuccess<T = unknown> {
    success: true;
    data: T;
    messageId?: string;
}

export interface MailerPluginErrorResult {
    success: false;
    error: MailerPluginError;
}

export interface MailerPluginError {
    message: string;
    statusCode: number;
    details?: unknown;
}

export type MailerPluginResult<T = unknown> = MailerPluginSuccess<T> | MailerPluginErrorResult;

export interface SendOtpOptions {
    to: string;
    recipientName: string;
    otp: string;
    expiresInMinutes?: number;
}

export interface SendForgotPasswordOptions {
    to: string;
    recipientName: string;
    resetLink: string;
    expiresInMinutes?: number;
}

export interface SendRawOptions {
    to: string | string[];
    subject: string;
    html: string;
    text?: string;
}

export interface SentMessageInfo {
    messageId: string;
    accepted: string[];
    rejected: string[];
}
