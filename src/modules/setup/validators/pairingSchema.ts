/**
 * 21.03 §02-setup (Part 02 handoff §9): the terminal-pairing form's client-side shape guard.
 * Mirrors `setup_pair_terminal`'s own validation texts exactly (`device.rs` §3.1) so the form shows
 * the same wording before the round trip that the backend would show after it — the backend is
 * still the authority (`core::device::pair_terminal`/`core::db::connect` do the real check), this
 * only catches an obviously-empty/out-of-range input before spending a network round trip.
 */
import { z } from 'zod';

export const pairingSchema = z.object({
  host: z.string().trim().min(1, 'أدخل اسم الجهاز الرئيسي أو عنوانه'),
  port: z
    .number({ error: 'رقم المنفذ غير صحيح' })
    .int('رقم المنفذ غير صحيح')
    .min(1, 'رقم المنفذ غير صحيح')
    .max(65535, 'رقم المنفذ غير صحيح'),
  code: z.string().trim().min(1, 'رمز الاقتران غير صحيح — أدخله كما يظهر على الجهاز الرئيسي'),
});

export type PairingFormValues = z.infer<typeof pairingSchema>;
