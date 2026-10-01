/**
 * The reference loads Handlebars files from `templates/mail/*.hbs` on disk. This project has no
 * `templates/` directory and no caller yet (this module is pre-built for a later phase — release
 * notifications), so templates are kept as small inline string builders instead of adding a new
 * on-disk template convention this task wasn't asked to design. A future phase can swap these for
 * file-based templates without changing `nodemailer.messages.ts`'s call sites.
 */

function appName(): string {
    return process.env.APP_NAME ?? "Equal";
}

export function renderOtpTemplate(vars: {
    recipientName: string;
    otp: string;
    expiresInMinutes?: number;
}): string {
    const { recipientName, otp, expiresInMinutes = 10 } = vars;
    return `
        <div style="font-family: sans-serif; direction: rtl; text-align: right;">
            <p>مرحبًا ${recipientName}،</p>
            <p>رمز التحقق الخاص بك هو:</p>
            <p style="font-size: 24px; font-weight: bold; letter-spacing: 4px;">${otp}</p>
            <p>صالح لمدة ${expiresInMinutes} دقيقة.</p>
            <p>${appName()}</p>
        </div>
    `.trim();
}

export function renderForgotPasswordTemplate(vars: {
    recipientName: string;
    resetLink: string;
    expiresInMinutes?: number;
}): string {
    const { recipientName, resetLink, expiresInMinutes = 30 } = vars;
    return `
        <div style="font-family: sans-serif; direction: rtl; text-align: right;">
            <p>مرحبًا ${recipientName}،</p>
            <p>لإعادة تعيين كلمة المرور، اضغط على الرابط التالي:</p>
            <p><a href="${resetLink}">${resetLink}</a></p>
            <p>ينتهي هذا الرابط خلال ${expiresInMinutes} دقيقة. إذا لم تطلب هذا، تجاهل هذه الرسالة.</p>
            <p>${appName()}</p>
        </div>
    `.trim();
}
