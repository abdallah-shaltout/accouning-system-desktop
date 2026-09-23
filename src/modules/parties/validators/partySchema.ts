import { z } from 'zod';

const optionalText = z.string().trim().optional();

/** Quick-add modal (docs/v2/08-customers-and-suppliers.md §1: "name + phone + type only"). */
export const partyQuickSchema = z.object({
  name: z.string().trim().min(2, 'الاسم مطلوب'),
  // Stored as E.164 by AppPhoneInput.
  phone: z.string().trim().regex(/^(\+\d{6,15})?$/, 'رقم هاتف غير صحيح').optional(),
  vatNumber: z
    .string()
    .trim()
    .regex(/^(3\d{13}3)?$/, '15 رقماً يبدأ وينتهي بالرقم 3')
    .optional(),
});

/** Back-compat alias — the old modal-only schema, now covering the quick-add fields. */
export const partySchema = z.object({
  name: z.string().trim().min(2, 'الاسم مطلوب'),
  phone: z.string().trim().regex(/^(\+\d{6,15})?$/, 'رقم هاتف غير صحيح').optional(),
  vatNumber: z
    .string()
    .trim()
    .regex(/^(3\d{13}3)?$/, '15 رقماً يبدأ وينتهي بالرقم 3')
    .optional(),
  address: optionalText,
  contactPerson: optionalText,
});

/** IBAN check (docs/v2/08 §1 "البنك"): SA = 24 characters total, starts with "SA". Mod-97 checksum. */
export function isValidIban(iban: string): boolean {
  const v = iban.replace(/\s/g, '').toUpperCase();
  if (!/^[A-Z]{2}\d{2}[A-Z0-9]+$/.test(v)) return false;
  if (v.startsWith('SA') && v.length !== 24) return false;
  const rearranged = v.slice(4) + v.slice(0, 4);
  const numeric = rearranged.replace(/[A-Z]/g, (c) => String(c.charCodeAt(0) - 55));
  // Mod-97 over a big numeric string, computed in chunks to stay within safe integer range.
  let remainder = 0;
  for (let i = 0; i < numeric.length; i += 7) {
    remainder = Number(`${remainder}${numeric.slice(i, i + 7)}`) % 97;
  }
  return remainder === 1;
}

/** Full party form (docs/v2/08 §1 sections: basics/contact/national-address/tax/terms/bank). */
export const partyFullSchema = z.object({
  name: z.string().trim().min(2, 'الاسم مطلوب'),
  nameEn: optionalText,
  type: z.enum(['individual', 'company']),
  email: z.string().trim().email('بريد إلكتروني غير صحيح').optional().or(z.literal('')),
  vatNumber: z
    .string()
    .trim()
    .regex(/^(3\d{13}3)?$/, '15 رقماً يبدأ وينتهي بالرقم 3')
    .optional(),
  crNumber: optionalText,
  nationalId: optionalText,
  paymentTermsDays: z.number().min(0, 'لا يقل عن 0').max(365, 'لا يزيد عن 365').optional(),
  creditLimit: z.number().min(0, 'لا يقل عن 0').optional(),
  iban: z
    .string()
    .trim()
    .refine((v) => !v || isValidIban(v), 'رقم آيبان غير صحيح')
    .optional(),
});
