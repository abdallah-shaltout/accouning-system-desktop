import { logger } from "@@shared/logger";
import { WhatsAppGowaPlugin } from "./whatsapp.plugin";
import { renderOtpMessage } from "./whatsapp.templates";

export { WhatsAppGowaPlugin } from "./whatsapp.plugin";
export type {
    GowaPluginConfig,
    GowaPluginResult,
    GowaPluginSuccess,
    GowaPluginErrorResult,
    GowaPluginError,
    GowaSendTextOptions,
    GowaSendImageOptions,
    GowaStatusResult,
} from "./whatsapp.types";
export { renderPlaceholder, renderOtpMessage } from "./whatsapp.templates";

let sharedPlugin: WhatsAppGowaPlugin | null = null;

function getPlugin(): WhatsAppGowaPlugin {
    if (!sharedPlugin) sharedPlugin = new WhatsAppGowaPlugin();
    return sharedPlugin;
}

/**
 * Sends a WhatsApp OTP code to `phone` via GOWA. `domains/otp/otp.service.ts` dynamically imports
 * this module and calls this export by name — see its `deliverOtp` helper, which falls back to a
 * console adapter if this import fails so the OTP domain is never blocked on this module landing.
 *
 * Per CLAUDE.md ("the free tier / dev setup must keep working without these external services"):
 * in `NODE_ENV=DEV`, or whenever `GOWA_BASE_URL` is unset (no credentials configured), this logs the
 * code to the console instead of making an HTTP call, so local dev never needs a real GOWA instance.
 */
export async function sendWhatsAppOtp(phone: string, code: string): Promise<void> {
    const noGowaConfigured = !process.env.GOWA_BASE_URL;

    if (process.env.NODE_ENV === "DEV" || noGowaConfigured) {
        // eslint-disable-next-line no-console
        console.log(`[DEV WHATSAPP OTP] ${phone}: ${code}`);
        return;
    }

    const result = await getPlugin().sendTextMessage({ to: phone, message: renderOtpMessage(code) });
    if (!result.success) {
        logger.error({ phone, error: result.error }, "whatsapp otp send failed");
        throw new Error(result.error.message || "Failed to send WhatsApp OTP");
    }
}
