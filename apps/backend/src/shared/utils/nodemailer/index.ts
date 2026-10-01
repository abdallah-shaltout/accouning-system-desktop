import { logger } from "@@shared/logger";
import { NodeMailerPlugin } from "./nodemailer.plugin";

export { NodeMailerPlugin } from "./nodemailer.plugin";
export type {
    MailerPluginConfig,
    MailerPluginResult,
    MailerPluginSuccess,
    MailerPluginErrorResult,
    MailerPluginError,
    SendOtpOptions,
    SendForgotPasswordOptions,
    SendRawOptions,
    SentMessageInfo,
} from "./nodemailer.types";

let sharedPlugin: NodeMailerPlugin | null = null;

function getPlugin(): NodeMailerPlugin {
    if (!sharedPlugin) sharedPlugin = new NodeMailerPlugin();
    return sharedPlugin;
}

/**
 * Not wired into any domain yet (pre-built for a later phase — release notifications, per the
 * Phase A task). `SMTP_HOST`/`SMTP_USER`/`SMTP_PASS` are not in `validateEnv.ts`'s schema, so this
 * server must keep working with none of them set: this helper no-ops (logs and returns) instead of
 * throwing when SMTP isn't configured, matching the WhatsApp module's dev-safety convention.
 */
export async function sendRawEmailSafe(options: {
    to: string | string[];
    subject: string;
    html: string;
    text?: string;
}): Promise<void> {
    if (!process.env.SMTP_HOST) {
        // eslint-disable-next-line no-console
        console.log(`[DEV EMAIL] to=${options.to} subject=${options.subject}`);
        return;
    }

    const result = await getPlugin().sendRaw(options);
    if (!result.success) {
        logger.error({ to: options.to, error: result.error }, "email send failed");
    }
}
