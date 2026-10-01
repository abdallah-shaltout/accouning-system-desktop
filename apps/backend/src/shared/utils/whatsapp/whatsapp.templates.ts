/**
 * GOWA has no server-side message template registry (unlike Meta's WhatsApp Cloud API, which the
 * reference's `whatsapp.templates.ts` queries via `/message_templates`). Messages are plain text
 * built on our side, so this file only keeps the placeholder-rendering helper the reference exposed
 * (`renderPlaceholder`), used for simple `{{1}}`-style substitution in canned message bodies.
 */
export function renderPlaceholder({ text, vars }: { text: string; vars: string[] }): string {
    return text.replace(/\{\{\s*(\d+)\s*\}\}/g, (_m, g1) => {
        const n = Number(g1);
        const v = vars[n - 1];
        return v !== undefined && v !== null ? v : "";
    });
}

/** Simple OTP message body — kept here so callers don't hand-roll the Arabic copy per call site. */
export function renderOtpMessage(code: string): string {
    return `رمز التحقق الخاص بك هو: ${code}\nصالح لمدة 5 دقائق.`;
}
