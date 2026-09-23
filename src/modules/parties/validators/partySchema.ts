import { z } from 'zod';

const optionalText = z.string().trim().optional();

export const partySchema = z.object({
  name: z.string().trim().min(2, 'الاسم مطلوب'),
  phone: z
    .string()
    .trim()
    .regex(/^(0\d{8,9})?$/, 'رقم هاتف سعودي (جوال 05xxxxxxxx أو ثابت 01xxxxxxx)')
    .optional(),
  vatNumber: z
    .string()
    .trim()
    .regex(/^(3\d{13}3)?$/, '15 رقماً يبدأ وينتهي بالرقم 3')
    .optional(),
  address: optionalText,
  contactPerson: optionalText,
});
